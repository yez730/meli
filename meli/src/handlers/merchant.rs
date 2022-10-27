use std::collections::HashMap;

use axum::{http::StatusCode, Json, extract::{Query, Path, State}};
use axum_session_authentication_middleware::{session::AuthSession , user as auth_user};
use axum_session_middleware::constants::session_keys;
use chrono::Local;
use email_address::EmailAddress;
use regex::Regex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::{
    schema::*,
    models::{Barber, NewUser, NewBarber, NewLoginInfo, Merchant, LoginInfo, Permission, Role}, authorization_policy,constant, regex_constants::CELLPHONE_REGEX_STRING
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
    
    let merchant_id=Uuid::parse_str(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();

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

    let merchants=barbers::table
        .inner_join(login_infos::table.on(login_infos::user_id.eq(barbers::user_id)))
        .inner_join(merchants::table.on(barbers::merchant_id.eq(merchants::merchant_id)))
        .filter(login_infos::login_info_account.eq(&query.login_account))
        .filter(login_infos::enabled.eq(true))
        .filter(barbers::enabled.eq(true))
        .filter(merchants::enabled.eq(true))
        .get_results::<(Barber,LoginInfo,Merchant)>(&mut *conn)
        .map(|bm|bm.into_iter().map(|t|t.2).collect())
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
    let merchant_id=Uuid::parse_str(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();

    let mut query=barbers::table
        .inner_join(merchants::table.on(barbers::merchant_id.eq(merchants::merchant_id)))
        .filter(merchants::enabled.eq(true))
        .filter(barbers::enabled.eq(true))
        .filter(barbers::merchant_id.eq(merchant_id))
        .into_boxed();

    if let Some(key)=search.key.as_ref(){
        query=query.filter(barbers::cellphone.ilike(format!("%{key}%")).or(barbers::real_name.ilike(format!("%{key}%"))));  
    }

    let data=query
        .order(barbers::create_time.desc())
        .get_results::<(Barber,Merchant)>(&mut *conn)
        .map(|bm|bm.into_iter().map(|(b,m)| BarberResponse { barber: b, merchant: m }).collect())
        .unwrap();
    
    Ok(Json(data))
}

pub async fn add_barber(
    State(pg):State<AxumPg>,
    auth: AuthSession<AxumPg, AxumPg,User>,
    Json(req): Json<BarberEditRequest>
)->Result<Json<BarberResponse>,(StatusCode,String)>{
    //检查登录&权限
    auth.require_permissions(vec![authorization_policy::MERCHANT_ADMINISTRATOR]).map_err(|e|(StatusCode::UNAUTHORIZED,e.to_string()))?;
    
    let mut conn=pg.pool.get().unwrap();
    
    let merchant_id=Uuid::parse_str(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();

    if req.cellphone.is_none()&&req.email.is_none(){
        return Err((StatusCode::BAD_REQUEST,"手机号码和邮箱不能同时为空".to_string()));
    }

    let mut existed_email_login_info_and_user=None;
    if req.email.is_some(){
        if !EmailAddress::is_valid(req.email.as_ref().unwrap()){
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
        if !Regex::new(CELLPHONE_REGEX_STRING).unwrap().is_match(req.cellphone.as_ref().unwrap()){
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
                    permissions:"[]",
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
                    permissions:"[]",
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
        cellphone:req.cellphone.as_deref(), //TODO nullable
        real_name:req.real_name.as_ref(),
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
    let mut ids=serde_json::from_str::<Vec<String>>(&user.as_ref().unwrap().permissions).unwrap();
    for permission_id in req.permission_ids{
        ids.push(format!("{}:{}",merchant_id,permission_id));
    }
    let barber_base_permission_id=permissions::table
        .filter(permissions::permission_code.eq(authorization_policy::BARBER_BASE)) 
        .filter(permissions::enabled.eq(true))
        .select(permissions::permission_id)
        .get_result::<Uuid>(&mut *conn)
        .unwrap();
    ids.push(format!("{}:{}",merchant_id,barber_base_permission_id));
    
    diesel::update(
        users::table
        .filter(users::user_id.eq(user.as_ref().unwrap().user_id))
        .filter(users::enabled.eq(true))
    )
    .set((
        users::permissions.eq(serde_json::to_string(&ids).unwrap()),
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

    let merchant_id=Uuid::parse_str(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();

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
    pub real_name:String,

    pub email:Option<String>,

    #[serde(rename ="permissionIds")]
    pub permission_ids:Vec<Uuid>,
}

pub async fn update_barber(
    State(pg):State<AxumPg>,
    Path(barber_id):Path<Uuid>, 
    auth: AuthSession<AxumPg, AxumPg,User>,
    Json(req): Json<BarberEditRequest>
)->Result<(),(StatusCode,String)>{
    //检查登录&权限
    auth.require_permissions(vec![authorization_policy::MERCHANT_ADMINISTRATOR]).map_err(|e|(StatusCode::UNAUTHORIZED,e.to_string()))?;

    let mut conn=pg.pool.get().unwrap();

    let merchant_id=Uuid::parse_str(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();
    
    if req.cellphone.is_none()&&req.email.is_none(){
        return Err((StatusCode::BAD_REQUEST,"手机号码和邮箱不能同时为空".to_string()));
    }

    let barber=barbers::table
        .filter(barbers::barber_id.eq(barber_id))
        .filter(barbers::merchant_id.eq(merchant_id))
        .filter(barbers::enabled.eq(true))
        .get_result::<Barber>(&mut *conn)
        .map_err(|_|{
            (StatusCode::NOT_FOUND,"理发师不存在".to_string())
        })?;

    let mut email_login_info=None;
    if req.email.is_some(){
        if !EmailAddress::is_valid(req.email.as_ref().unwrap()){
            return Err((StatusCode::BAD_REQUEST,"邮箱格式不正确".to_string()));
        } 

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
        if !Regex::new(CELLPHONE_REGEX_STRING).unwrap().is_match(req.cellphone.as_ref().unwrap()){
            return Err((StatusCode::BAD_REQUEST,"手机号码格式不正确".to_string()));
        }

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
    let administrator_permission_id=permissions::table
        .filter(permissions::permission_code.eq(authorization_policy::MERCHANT_ADMINISTRATOR)) 
        .filter(permissions::enabled.eq(true))
        .select(permissions::permission_id)
        .get_result::<Uuid>(&mut *conn)
        .unwrap();

    let permissions=users::table
        .filter(users::enabled.eq(true))
        .filter(users::user_id.eq(barber.user_id))
        .select(users::permissions)
        .get_result::<String>(&mut *conn)
        .unwrap();

    let mut ids=serde_json::from_str::<Vec<String>>(permissions.as_str()).unwrap();
    let is_administrator=ids.contains(&format!("{}:{}",merchant_id,administrator_permission_id));
    if !is_administrator{
        tracing::debug!("is_administrator");
        ids.retain(|id_str|{
            let mut id_iter=id_str.split(':'); // `merchant_id:permission_id` or `permission_id`
    
            let id=Uuid::parse_str(id_iter.next().unwrap()).unwrap();
            merchant_id!=id
        });
        
        for permission_id in req.permission_ids{
            ids.push(format!("{}:{}",merchant_id,permission_id));
        }
        let barber_base_permission_id=permissions::table
            .filter(permissions::permission_code.eq(authorization_policy::BARBER_BASE)) 
            .filter(permissions::enabled.eq(true))
            .select(permissions::permission_id)
            .get_result::<Uuid>(&mut *conn)
            .unwrap();
        ids.push(format!("{}:{}",merchant_id,barber_base_permission_id));
        
        diesel::update(
            users::table
            .filter(users::user_id.eq(barber.user_id))
            .filter(users::enabled.eq(true))
        )
        .set((
                users::permissions.eq(serde_json::to_string(&ids).unwrap()),
                users::update_time.eq(Local::now())
            ))
        .execute(&mut *conn)
        .unwrap();
    }

    // 3. 刷新 session data (如果登录了的话)
    refresh_identity_data(barber.user_id, merchant_id, pg).await;
    
    Ok(())
}

async fn refresh_identity_data(user_id:Uuid,merchant_id:Uuid, pg:AxumPg){
    let mut conn=pg.pool.get().unwrap();

    let data_str=sessions::table
        .filter(sessions::user_id.eq(user_id))
        .filter(sessions::expiry_time.gt(Local::now()))
        .select(sessions::data)
        .get_result::<String>(&mut *conn)
        .ok();

    if let Some(data)=data_str{
        let user=users::table
        .filter(users::user_id.eq(user_id))
        .filter(users::enabled.eq(true))
        .get_result::<User>(&mut *conn)
        .unwrap();

        let mut current_permission_ids=Vec::new();
        let ids=serde_json::from_str::<Vec<String>>(&user.permissions).unwrap();
        for id_str in ids {
            let mut id_iter=id_str.split(':');

            let id1=Uuid::parse_str(id_iter.next().unwrap()).unwrap();
            let id2=id_iter.next();
            
            match id2 {
                Some(permission_id) if id1==merchant_id=>{
                    current_permission_ids.push(Uuid::parse_str(permission_id).unwrap());
                }
                None=>{
                    current_permission_ids.push(id1);
                }
                _=>{}
            }
        }

        let permissions=permissions::table
            .filter(permissions::permission_id.eq_any(current_permission_ids))
            .filter(permissions::enabled.eq(true))
            .get_results::<Permission>(&mut *conn)
            .unwrap();
        let roles=roles::table
            .filter(roles::role_id.eq_any(serde_json::from_str::<Vec<Uuid>>(&user.roles).unwrap())) 
            .filter(roles::enabled.eq(true))
            .get_results::<Role>(&mut *conn)
            .unwrap();

        let identity=auth_user::Identity{
            user_id:user.user_id,
            roles:roles.into_iter().map(|r|auth_user::Role{
                role_id: r.role_id,
                role_code: r.role_code,
                role_name:r.role_name,

                permissions:r.permissions,
                description:r.description,
                enabled:r.enabled,
                create_time: r.create_time,
                update_time: r.update_time,
                data: r.data,
            }).collect(),
            permission_codes:permissions.iter().map(|p|p.permission_code.clone()).collect(),
            permissions:permissions.into_iter().map(|p|auth_user::Permission{
                permission_id: p.permission_id,
                    permission_code: p.permission_code,
                    permission_name :p.permission_name,
                    description: p.description,
                    enabled:p.enabled,
                    create_time: p.create_time,
                    update_time: p.update_time,
                    data: p.data,
            }).collect(),
        };

        let mut session_data:HashMap<String,String>=serde_json::from_str::<HashMap<String,String>>(&data).unwrap();
        session_data.insert(session_keys::IDENTITY.to_string(), serde_json::to_string(&identity).unwrap());

        diesel::update(
            sessions::table
            .filter(sessions::user_id.eq(user_id))
            .filter(sessions::expiry_time.gt(Local::now()))
        )
        .set((
                sessions::data.eq(serde_json::to_string(&session_data).unwrap()),                    
                sessions::update_time.eq(Local::now()),
            ))
        .execute(&mut *conn)
        .unwrap();
    }
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

    let merchant_id=Uuid::parse_str(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();

    let (barber,user)=barbers::table.inner_join(users::table.on(barbers::user_id.eq(users::user_id)))
        .filter(users::enabled.eq(true))
        .filter(barbers::barber_id.eq(barber_id))
        .filter(barbers::merchant_id.eq(merchant_id))
        .filter(barbers::enabled.eq(true))
        .get_result::<(Barber,User)>(&mut *conn)
        .map_err(|_|{
            (StatusCode::NOT_FOUND,"理发师不存在".to_string())
        })?;

    let mut permission_codes=Vec::new();

    let ids=serde_json::from_str::<Vec<String>>(&user.permissions).unwrap();
    for id_str in ids {
        let mut id_iter=id_str.split(':'); // `merchant_id:permission_id` or `permission_id`

        let id=Uuid::parse_str(id_iter.next().unwrap()).unwrap();
        if id==merchant_id {
            let permission_id=Uuid::parse_str(id_iter.next().unwrap()).unwrap();

            let permission_code=permissions::table
                .filter(permissions::permission_id.eq(permission_id)) 
                .filter(permissions::enabled.eq(true))
                .select(permissions::permission_code)
                .get_result::<String>(&mut *conn)
                .unwrap();
            permission_codes.push(permission_code);
        }
    }

    let res= BarberEditResponse{
        barber,
        permission_codes
    };
        
    Ok(Json(res))
}

#[derive(Serialize)]
pub struct PermissionResponse{
    #[serde(rename ="allPermissions")]
    pub all_permissions:Vec<Permission>,

    #[serde(rename ="defaultPermissions")]
    pub default_permissions:Vec<Permission>,
}

pub async fn get_all_permissions(
    State(pg):State<AxumPg>,
    auth: AuthSession<AxumPg, AxumPg,User>,
)->Result<Json<PermissionResponse>,(StatusCode,String)>{
    //检查登录&权限
    auth.require_permissions(vec![authorization_policy::MERCHANT_ADMINISTRATOR]).map_err(|e|(StatusCode::UNAUTHORIZED,e.to_string()))?;
    
    let mut conn=pg.pool.get().unwrap();

    let all_permissions=permissions::table
        .filter(permissions::permission_code.eq_any(authorization_policy::ADMINISTRATOR_PERMISSIONS_OF_MERCHANT_BARBER)) 
        .filter(permissions::enabled.eq(true))
        .get_results::<Permission>(&mut *conn)
        .unwrap();

    let default_permissions=permissions::table
        .filter(permissions::permission_code.eq_any(authorization_policy::DEFAULT_PERMISSIONS_OF_MERCHANT_BARBER)) 
        .filter(permissions::enabled.eq(true))
        .get_results::<Permission>(&mut *conn)
        .unwrap();

    let res=PermissionResponse{
        all_permissions,
        default_permissions
    };
    Ok(Json(res))
}
