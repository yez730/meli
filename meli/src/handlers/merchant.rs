use axum::{http::StatusCode, Json, extract::{Query, Path, State}};
use axum_session_authentication_middleware::session::AuthSession;
use chrono::Local;
use email_address::EmailAddress;
use regex::Regex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::{
    schema::*,
    models::{Barber, NewUser, NewBarber, NewLoginInfo, Merchant, LoginInfo, Permission}, authorization_policy,constant, regex_constants::CELLPHONE_REGEX_STRING
};
use diesel::{
    prelude::*, // for .filter
    select, 
    dsl::exists,
};
use crate::{models::User, axum_pg::AxumPg};
use super::{Search, barber::BarberResponse};

pub async fn get_current_merchant(State(pg):State<AxumPg>,
    auth: AuthSession<AxumPg, AxumPg,User>,
    )->Result<Json<Merchant>,(StatusCode,String)>{
    //检查登录&权限
    auth.require_permissions(vec![authorization_policy::MERCHANT_ADMINISTRATOR]).map_err(|e|(StatusCode::UNAUTHORIZED,e.to_string()))?;

    let mut conn=pg.pool.get().unwrap();
    
    let merchant_id=serde_json::from_str::<Uuid>(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();

    let merchant=merchants::table
        .filter(merchants::enabled.eq(true))
        .filter(merchants::merchant_id.eq(merchant_id))
        .get_result::<Merchant>(&mut *conn)
        .unwrap();

    Ok(Json(merchant))
}

#[derive(Deserialize)]
pub struct GetMerchantsByLoginAccount{
    #[serde(rename ="loginAccount")]
    login_account:String,
}

pub async fn get_merchants_by_login_account(State(pg):State<AxumPg>,  Query(query):Query<GetMerchantsByLoginAccount>,)->Result<Json<Vec<Merchant>>,(StatusCode,String)>{
    let mut conn=pg.pool.get().unwrap();

    let login_info=login_infos::table
        .filter(login_infos::login_info_account.eq(&query.login_account))
        .filter(login_infos::enabled.eq(true))
        .get_result::<LoginInfo>(&mut *conn)
        .ok();

    if login_info.is_none() {
        tracing::warn!("通过登录名获取商户列表失败，登录名：{}。",query.login_account);
        return Err((StatusCode::NOT_FOUND,"用户未注册".to_string()));
    }

    let merchants=barbers::table
        .inner_join(merchants::table.on(barbers::merchant_id.eq(merchants::merchant_id)))
        .filter(barbers::enabled.eq(true))
        .filter(barbers::user_id.eq(login_info.unwrap().user_id))
        .filter(merchants::enabled.eq(true))
        .get_results::<(Barber,Merchant)>(&mut *conn)
        .map(|bm|bm.into_iter().map(|t|t.1).collect())
        .unwrap();

    return Ok(Json(merchants));
}

pub async fn get_barbers(
    State(pg):State<AxumPg>,
    Query(search):Query<Search>, 
    auth: AuthSession<AxumPg, AxumPg,User>,
)->Result<Json<Vec<BarberResponse>>,(StatusCode,String)>{
    //检查登录&权限
    auth.require_permissions(vec![authorization_policy::MERCHANT_ADMINISTRATOR]).map_err(|e|(StatusCode::UNAUTHORIZED,e.to_string()))?;
    
    let mut conn=pg.pool.get().unwrap();

    let merchant_id=serde_json::from_str::<Uuid>(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();

    let mut query=barbers::table
        .inner_join(merchants::table.on(barbers::merchant_id.eq(merchants::merchant_id)))
        .filter(merchants::enabled.eq(true))
        .filter(barbers::enabled.eq(true))
        .filter(barbers::merchant_id.eq(merchant_id))
        .into_boxed();

    if let Some(key)=search.key.as_ref(){
        if key.len()>0 {
            query=query.filter(barbers::cellphone.ilike(format!("%{key}%")).or(barbers::real_name.ilike(format!("%{key}%"))));   
        }
    }

    let data=query
        .order(barbers::create_time.desc())
        .get_results::<(Barber,Merchant)>(&mut *conn)
        .map(|bm|bm.into_iter().map(|(b,m)| BarberResponse { barber: b, merchant: m }).collect())
        .unwrap();
    
    Ok(Json(data))
}

#[derive(Deserialize)]
pub struct BarberAddRequest{
    pub cellphone:Option<String>,

    #[serde(rename ="realName")]
    pub real_name:Option<String>,

    pub email:Option<String>,

    #[serde(rename ="permissionCodes")]
    pub permission_codes:Vec<String>,
}

pub async fn add_barber(
    State(pg):State<AxumPg>,
    auth: AuthSession<AxumPg, AxumPg,User>,
    Json(req): Json<BarberAddRequest>
)->Result<Json<BarberResponse>,(StatusCode,String)>{
    //检查登录&权限
    auth.require_permissions(vec![authorization_policy::MERCHANT_ADMINISTRATOR]).map_err(|e|(StatusCode::UNAUTHORIZED,e.to_string()))?;
    
    let mut conn=pg.pool.get().unwrap();
    
    let merchant_id=serde_json::from_str::<Uuid>(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();

    if req.cellphone.is_none()&&req.email.is_none(){
        return Err((StatusCode::BAD_REQUEST,"手机号码和邮箱不能同时为空".to_string()));
    }

    let mut existed_email_login_info_and_user=None;
    if req.email.is_some(){
        if EmailAddress::is_valid(req.email.as_ref().unwrap()){
            return Err((StatusCode::BAD_REQUEST,"邮箱格式不正确".to_string()));
        } 

        existed_email_login_info_and_user=login_infos::table.inner_join(users::table.on(login_infos::user_id.eq(users::user_id)))
            .filter(users::enabled.eq(true))
            .filter(login_infos::enabled.eq(true))
            .filter(login_infos::login_info_type.eq("Email"))
            .filter(login_infos::login_info_account.eq(req.email.as_ref().unwrap()))
            .get_result::<(LoginInfo,User)>(&mut *conn)
            .ok();
        if let Some((login_info,_))=existed_email_login_info_and_user.as_ref(){
            let existed_email=select(exists(
                barbers::table
                .filter(barbers::enabled.eq(true))
                .filter(barbers::merchant_id.eq(merchant_id))
                .filter(barbers::user_id.eq(login_info.user_id))
            ))
            .get_result(&mut *conn)
            .unwrap();

            if existed_email{
                return Err((StatusCode::BAD_REQUEST,"该商户已添加该邮箱的理发师".to_string())); //TODO 验证email唯一性
            }
        }
    }

    let mut existed_cellphone_login_info_and_user=None;
    if req.cellphone.is_some(){
        if Regex::new(CELLPHONE_REGEX_STRING).unwrap().is_match(req.cellphone.as_ref().unwrap()){
            return Err((StatusCode::BAD_REQUEST,"手机号码格式不正确".to_string()));
        }
        
        existed_cellphone_login_info_and_user=login_infos::table.inner_join(users::table.on(login_infos::user_id.eq(users::user_id)))
            .filter(users::enabled.eq(true))
            .filter(login_infos::enabled.eq(true))
            .filter(login_infos::login_info_type.eq("Cellphone"))
            .filter(login_infos::login_info_account.eq(req.cellphone.as_ref().unwrap()))
            .get_result::<(LoginInfo,User)>(&mut *conn)
            .ok();
        if let Some((login_info,_))=existed_cellphone_login_info_and_user.as_ref(){
            let existed_cellphone=select(exists(
                barbers::table
                .filter(barbers::enabled.eq(true))
                .filter(barbers::merchant_id.eq(merchant_id))
                .filter(barbers::user_id.eq(login_info.user_id))
            ))
            .get_result(&mut *conn)
            .unwrap();

            if existed_cellphone{
                return Err((StatusCode::BAD_REQUEST,"该商户已添加该手机号码的理发师".to_string())); //TODO 验证email唯一性
            }
        }
    }
    
    let mut user:Option<User>=None;

    if req.email.is_some(){
        if existed_email_login_info_and_user.is_none(){
            if user.is_none(){
                let new_user=NewUser{
                    user_id: &Uuid::new_v4(),
                    description: "商户管理员添加",
                    permissions:&serde_json::to_string(authorization_policy::DEFAULT_PERMISSIONS_OF_MERCHANT_BARBER).unwrap(),
                    roles:"[]",
                    enabled:true,
                    create_time: Local::now(),
                    update_time: Local::now(),
                    data: None,
                };
                let res=diesel::insert_into(users::table)
                .values(&new_user)
                .get_result(&mut *conn)
                .unwrap();

                user=Some(res);
            }
            
            let login_info=NewLoginInfo{
                login_info_id: &Uuid::new_v4(),
                login_info_account: &req.email.as_ref().unwrap(),
                login_info_type: "Email", 
                user_id: &user.as_ref().unwrap().user_id,
                enabled: true, 
                create_time: Local::now(),
                update_time: Local::now(),
            };
            diesel::insert_into(login_infos::table)
                .values(&login_info)
                .execute(&mut *conn)
                .unwrap();
        }
    }

    if req.cellphone.is_some(){
        if existed_cellphone_login_info_and_user.is_none(){
            if user.is_none(){
                let new_user=NewUser{
                    user_id: &Uuid::new_v4(),
                    description: "商户管理员添加",
                    permissions:&serde_json::to_string(authorization_policy::DEFAULT_PERMISSIONS_OF_MERCHANT_BARBER).unwrap(),
                    roles:"[]",
                    enabled:true,
                    create_time: Local::now(),
                    update_time: Local::now(),
                    data: None,
                };
                let res=diesel::insert_into(users::table)
                .values(&new_user)
                .get_result(&mut *conn)
                .unwrap();

                user=Some(res);
            }
            
            let login_info=NewLoginInfo{
                login_info_id: &Uuid::new_v4(),
                login_info_account: &req.cellphone.as_ref().unwrap(),
                login_info_type: "Cellphone",
                user_id: &user.as_ref().unwrap().user_id,
                enabled: true, 
                create_time: Local::now(),
                update_time: Local::now(),
            };
            diesel::insert_into(login_infos::table)
            .values(&login_info)
            .execute(&mut *conn)
            .unwrap();
        }
    }

    let new_barber=NewBarber{
        user_id: &user.as_ref().unwrap().user_id,
        barber_id: &Uuid::new_v4(),
        merchant_id:&merchant_id,
        email:req.email.as_deref(),
        cellphone:&req.cellphone.unwrap(), //TODO nullable
        real_name:req.real_name.as_deref(),
        enabled:true,
        create_time: Local::now(),
        update_time: Local::now(),
        data: None,
    };
    let barber=diesel::insert_into(barbers::table)
        .values(&new_barber)
        .get_result(&mut *conn)
        .unwrap();

    // add permissions
    let permissions=permissions::table
        .filter(permissions::permission_id.eq_any(serde_json::from_str::<Vec<Uuid>>(user.as_ref().unwrap().permissions.as_str()).unwrap())) 
        .filter(permissions::enabled.eq(true))
        .get_results::<Permission>(&mut *conn)
        .unwrap();
    let mut permission_ids=permissions.iter().map(|p|p.permission_id).collect::<Vec<_>>();
    for &permission_code in authorization_policy::DEFAULT_PERMISSIONS_OF_MERCHANT_BARBER{
        if !permissions.iter().any(|p|p.permission_code==permission_code){
            let permission_id=permissions::table
            .filter(permissions::permission_code.eq(permission_code)) 
            .filter(permissions::enabled.eq(true))
            .select(permissions::permission_id)
            .get_result::<Uuid>(&mut *conn)
            .unwrap();

            permission_ids.push(permission_id);
        }
    }
    diesel::update(
        users::table
        .filter(users::user_id.eq(user.as_ref().unwrap().user_id))
        .filter(users::enabled.eq(true))
    )
    .set((
        users::permissions.eq(serde_json::to_string(&permission_ids).unwrap()),
        users::update_time.eq(Local::now())
    ))
    .execute(&mut *conn)
    .unwrap();

    // TODO 发生加入连接， 自己设置密码
    // let salt=env::var("DATABASE_ENCRYPTION_SAULT").unwrap();
    // let config = argon2::Config::default();
    // let hash = argon2::hash_encoded(req.password.as_bytes(), salt.as_bytes(), &config).unwrap();
    // let new_password_login_provider=NewPasswordLoginProvider{ //TODO 一个
    //     user_id: &user.as_ref().unwrap().user_id,
    //     password_hash: &hash,
    //     enabled:true,
    //     create_time: Local::now(),
    //     update_time: Local::now(),
    //     data:None
    // };
    // diesel::insert_into(password_login_providers::table)
    //     .values(&new_password_login_provider)
    //     .execute(&mut *conn).map_err(|e|{
    //         tracing::error!("{}",e.to_string());
    //         (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
    //     })?;

    let merchant=merchants::table
        .filter(merchants::enabled.eq(true))
        .filter(merchants::merchant_id.eq(merchant_id))
        .get_result(&mut *conn)
        .unwrap();

    let barber_response=BarberResponse{barber,merchant};

    Ok(Json(barber_response))
}

pub async fn delete_barber(
    State(pg):State<AxumPg>,
    Path(barber_id):Path<Uuid>, 
    auth: AuthSession<AxumPg, AxumPg,User>,
)->Result<(),(StatusCode,String)>{
    //检查登录&权限
    auth.require_permissions(vec![authorization_policy::MERCHANT_ADMINISTRATOR]).map_err(|e|(StatusCode::UNAUTHORIZED,e.to_string()))?;
    
    let mut conn=pg.pool.get().unwrap();

    let merchant_id=serde_json::from_str::<Uuid>(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();

    let _existed=barbers::table
        .filter(barbers::enabled.eq(true))
        .filter(barbers::barber_id.eq(barber_id))
        .filter(barbers::merchant_id.eq(merchant_id))
        .get_result::<Barber>(&mut *conn)
        .map_err(|_|{
            (StatusCode::NOT_FOUND,"理发师不存在".to_string())
        })?;

    diesel::update(
        barbers::table
        .filter(barbers::barber_id.eq(barber_id))
        .filter(barbers::merchant_id.eq(merchant_id))
        .filter(barbers::enabled.eq(true))
    )
    .set((
        barbers::enabled.eq(false),
        barbers::update_time.eq(Local::now())
    ))
    .execute(&mut *conn)
    .unwrap();

    //TODO 删除perssmission/login_info/user/password 

    Ok(())
}

#[derive(Deserialize)]
pub struct BarberEditRequest{
    pub cellphone:Option<String>,

    #[serde(rename ="realName")]
    pub real_name:Option<String>,

    pub email:Option<String>,

    #[serde(rename ="permissionCodes")]
    pub permission_codes:Vec<String>,
}

pub async fn update_barber(
    State(pg):State<AxumPg>,
    Path(barber_id):Path<Uuid>, 
    mut auth: AuthSession<AxumPg, AxumPg,User>,
    Json(req): Json<BarberEditRequest>
)->Result<(),(StatusCode,String)>{
    //检查登录&权限
    auth.require_permissions(vec![authorization_policy::MERCHANT_ADMINISTRATOR]).map_err(|e|(StatusCode::UNAUTHORIZED,e.to_string()))?;

    let mut conn=pg.pool.get().unwrap();

    let merchant_id=serde_json::from_str::<Uuid>(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();
    
    if req.cellphone.is_none()&&req.email.is_none(){
        return Err((StatusCode::BAD_REQUEST,"手机号码和邮箱不能同时为空".to_string()));
    }

    let (barber,user)=barbers::table.inner_join(users::table.on(barbers::user_id.eq(users::user_id)))
        .filter(users::enabled.eq(true))
        .filter(barbers::barber_id.eq(barber_id))
        .filter(barbers::merchant_id.eq(merchant_id))
        .filter(barbers::enabled.eq(true))
        .get_result::<(Barber,User)>(&mut *conn)
        .map_err(|_|{
            (StatusCode::NOT_FOUND,"理发师不存在".to_string())
        })?;

    let mut email_login_info=None;
    if req.email.is_some(){
        email_login_info=login_infos::table
            .filter(login_infos::enabled.eq(true))
            .filter(login_infos::login_info_type.eq("Email"))
            .filter(login_infos::login_info_account.eq(req.email.as_ref().unwrap()))
            .get_result::<LoginInfo>(&mut *conn)
            .ok();
        if let Some(login_info)=email_login_info.as_ref(){
            if login_info.user_id!=barber.user_id{
                return Err((StatusCode::BAD_REQUEST,"该邮箱已被其他用户使用".to_string())); //TODO 添加后验证email有效性
            }
        }
    }

    let mut cellphone_login_info=None;
    if req.cellphone.is_some(){
        cellphone_login_info=login_infos::table
            .filter(login_infos::enabled.eq(true))
            .filter(login_infos::login_info_type.eq("Cellphone"))
            .filter(login_infos::login_info_account.eq(req.cellphone.as_ref().unwrap()))
            .get_result::<LoginInfo>(&mut *conn)
            .ok();
        if let Some(login_info)=cellphone_login_info.as_ref(){
            if login_info.user_id!=barber.user_id{
                return Err((StatusCode::BAD_REQUEST,"该手机号已被其他用户使用".to_string())); //TODO 添加后验证email有效性
            }
        }
    }

    if req.email.is_some(){
        if email_login_info.is_none(){
            let login_info=NewLoginInfo{
                login_info_id: &Uuid::new_v4(),
                login_info_account: &req.email.as_ref().unwrap(),
                login_info_type: "Email", 
                user_id: &barber.user_id,
                enabled: true, 
                create_time: Local::now(),
                update_time: Local::now(),
            };
            diesel::insert_into(login_infos::table)
                .values(&login_info)
                .execute(&mut *conn)
                .unwrap();
        }
    } 

    if req.cellphone.is_some(){
        if cellphone_login_info.is_none(){
            let login_info=NewLoginInfo{
                login_info_id: &Uuid::new_v4(),
                login_info_account: &req.cellphone.as_ref().unwrap(),
                login_info_type: "Cellphone",
                user_id: &barber.user_id,
                enabled: true, 
                create_time: Local::now(),
                update_time: Local::now(),
            };
            diesel::insert_into(login_infos::table)
                .values(&login_info)
                .execute(&mut *conn)
                .unwrap();
        }
    }

    diesel::update(
        barbers::table
        .filter(barbers::barber_id.eq(barber_id))
        .filter(barbers::enabled.eq(true))
    )
    .set((
        barbers::cellphone.eq(req.cellphone.unwrap()),
        barbers::real_name.eq(req.real_name),
        barbers::email.eq(req.email),
        barbers::update_time.eq(Local::now())
    ))
    .execute(&mut *conn)
    .unwrap();

    // update permissions
    // 1. 删掉旧的
    diesel::update(
        users::table
        .filter(users::user_id.eq(user.user_id))
        .filter(users::enabled.eq(true))
    )
    .set((
        users::permissions.eq("[]"),
    )).execute(&mut *conn).unwrap();
    // 2. 添加新权限
    let mut permission_ids=Vec::new();
    for permission_code in req.permission_codes{
        let permission_id=permissions::table
            .filter(permissions::permission_code.eq(permission_code)) 
            .filter(permissions::enabled.eq(true))
            .select(permissions::permission_id)
            .get_result::<Uuid>(&mut *conn)
            .ok();
        if permission_id.is_some(){
            permission_ids.push(permission_id);
        }
    }
    diesel::update(
        users::table
        .filter(users::user_id.eq(user.user_id))
        .filter(users::enabled.eq(true))
    )
    .set((
            users::permissions.eq(serde_json::to_string(&permission_ids).unwrap()),
            users::update_time.eq(Local::now())
        ))
    .execute(&mut *conn)
    .unwrap();
    // 3. 刷新identity
    auth.refresh_identity(user.user_id).await;
    
    Ok(())
}

#[derive(Serialize)]
pub struct BarberEditResponse{
    #[serde(flatten)]
    pub barber:Barber,

    #[serde(rename ="permissionCodes")]
    pub permission_codes:Vec<String>,
}

pub async fn get_barber(
    State(pg):State<AxumPg>,
    Path(barber_id):Path<Uuid>, 
    auth: AuthSession<AxumPg, AxumPg,User>,
)->Result<Json<BarberEditResponse>,(StatusCode,String)>{
    //检查登录&权限
    auth.require_permissions(vec![authorization_policy::MERCHANT_ADMINISTRATOR]).map_err(|e|(StatusCode::UNAUTHORIZED,e.to_string()))?;

    let mut conn=pg.pool.get().unwrap();

    let merchant_id=serde_json::from_str::<Uuid>(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();

    let (barber,user)=barbers::table.inner_join(users::table.on(barbers::user_id.eq(users::user_id)))
        .filter(users::enabled.eq(true))
        .filter(barbers::barber_id.eq(barber_id))
        .filter(barbers::merchant_id.eq(merchant_id))
        .filter(barbers::enabled.eq(true))
        .get_result::<(Barber,User)>(&mut *conn)
        .map_err(|_|{
            (StatusCode::NOT_FOUND,"理发师不存在".to_string())
        })?;

    let permission_codes=permissions::table
        .filter(permissions::permission_id.eq_any(serde_json::from_str::<Vec<Uuid>>(&user.permissions).unwrap())) 
        .filter(permissions::enabled.eq(true))
        .select(permissions::permission_code)
        .get_results::<String>(&mut *conn)
        .unwrap();

    let res= BarberEditResponse{
        barber,
        permission_codes
    };
        
    Ok(Json(res))
}
