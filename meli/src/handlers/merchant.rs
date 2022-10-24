use std::env;

use axum::{http::StatusCode, Json, extract::{Query, Path, State}};
use axum_session_authentication_middleware::session::AuthSession;
use chrono::{Local, NaiveDate};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::{
    schema::*,
    models::{Barber, NewUser, NewBarber, NewLoginInfo, NewPasswordLoginProvider, Merchant, LoginInfo, Permission}, authorization_policy,constant
};
use diesel::{
    prelude::*, // for .filter
    select, 
    dsl::exists,
};
use crate::{models::User, axum_pg_pool::AxumPgPool};
use super::{Search, barber::BarberResponse};

pub async fn get_current_merchant(State(pool):State<AxumPgPool>,
    Query(search):Query<Search>, 
    auth: AuthSession<AxumPgPool, AxumPgPool,User>,
    )->Result<Json<Merchant>,(StatusCode,String)>{
    //检查登录
    let _=auth.identity.as_ref().ok_or((StatusCode::UNAUTHORIZED,"no login".to_string()))?;

    //检查权限
    auth.require_permissions(vec![authorization_policy::MERCHANT_ADMINISTRATOR])
        .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"no permission".to_string()))?;

    let mut conn=pool.pool.get().unwrap();//TODO error
    
    let merchant_id=serde_json::from_str::<Uuid>(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID))
        .unwrap();

    let merchant=merchants::table
        .filter(merchants::enabled.eq(true))
        .filter(merchants::merchant_id.eq(merchant_id))
        .get_result::<Merchant>(&mut *conn)
        .map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;

    Ok(Json(merchant))
}

#[derive(Deserialize)]
pub struct GetMerchantsByLoginAccount{
    login_account:String,
}

pub async fn get_merchants_by_login_account(State(pool):State<AxumPgPool>,  Query(query):Query<GetMerchantsByLoginAccount>,)->Result<Json<Vec<Merchant>>,(StatusCode,String)>{
    let mut conn=pool.pool.get().unwrap();//TODO error

    let login_info=login_infos::table
        .filter(login_infos::login_info_account.eq(query.login_account.clone()))
        .filter(login_infos::enabled.eq(true))
        .get_result::<LoginInfo>(&mut *conn)
        .ok();

    if login_info.is_none() {
        tracing::warn!("通过登录名获取商户列表失败，登录名：{}。",query.login_account);
        return Err((StatusCode::INTERNAL_SERVER_ERROR,"用户未注册".to_string()));
    }

    let merchants=barbers::table
        .inner_join(merchants::table.on(barbers::merchant_id.eq(merchants::merchant_id)))
        .filter(barbers::enabled.eq(true))
        .filter(barbers::user_id.eq(login_info.unwrap().user_id))
        .filter(merchants::enabled.eq(true))
        .get_results::<(Barber,Merchant)>(&mut *conn)
        .map(|bm|bm.into_iter().map(|t|t.1).collect())
        .map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;

    return Ok(Json(merchants));
}

pub async fn get_barbers(
    State(pool):State<AxumPgPool>,
    Query(search):Query<Search>, 
    auth: AuthSession<AxumPgPool, AxumPgPool,User>,
)->Result<Json<Vec<BarberResponse>>,(StatusCode,String)>{
    //检查登录
    let _=auth.identity.as_ref().ok_or((StatusCode::UNAUTHORIZED,"no login".to_string()))?;

    //检查权限
    auth.require_permissions(vec![authorization_policy::MERCHANT_ADMINISTRATOR])
        .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"no permission".to_string()))?;
    
    let mut conn=pool.pool.get().unwrap();//TODO error

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
        .map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;
    
    Ok(Json(data))
}

pub async fn add_barber(
    State(pool):State<AxumPgPool>,
    auth: AuthSession<AxumPgPool, AxumPgPool,User>,
    Json(req): Json<BarberEditRequest>
)->Result<Json<BarberResponse>,(StatusCode,String)>{
    //检查登录
    let _=auth.identity.as_ref().ok_or((StatusCode::UNAUTHORIZED,"no login".to_string()))?;

    //检查权限
    auth.require_permissions(vec![authorization_policy::MERCHANT_ADMINISTRATOR])
        .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"no permission".to_string()))?;
    
    let mut conn=pool.pool.get().unwrap();//TODO error
    
    let merchant_id=serde_json::from_str::<Uuid>(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();

    if req.cellphone.is_none()&&req.email.is_none(){
        return Err((StatusCode::BAD_REQUEST,"手机号码和邮箱不能同时为空".to_string()));
    }

    if req.email.is_some(){
        let email_login_info=login_infos::table
            .filter(login_infos::enabled.eq(true))
            .filter(login_infos::login_info_type.eq("Email"))
            .filter(login_infos::login_info_account.eq(req.email.clone().unwrap()))
            .get_result::<LoginInfo>(&mut *conn)
            .ok();
        if let Some(login_info)=email_login_info{
            let existed_email=select(exists(
                barbers::table
                .filter(barbers::enabled.eq(true))
                .filter(barbers::merchant_id.eq(merchant_id))
                .filter(barbers::user_id.eq(login_info.user_id))
            ))
            .get_result(&mut *conn)
            .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"get_result error".to_string()))?;
            if existed_email{
                return Err((StatusCode::BAD_REQUEST,"该商户已添加该邮箱的理发师".to_string())); //TODO 添加后验证email有效性
            }
        }
    }

    if req.cellphone.is_some(){
        let cellphone_login_info=login_infos::table
        .filter(login_infos::enabled.eq(true))
        .filter(login_infos::login_info_type.eq("Cellphone"))
        .filter(login_infos::login_info_account.eq(req.cellphone.clone().unwrap()))
        .get_result::<LoginInfo>(&mut *conn)
        .ok();
        if let Some(login_info)=cellphone_login_info{
            let existed_cellphone=select(exists(
                barbers::table
                .filter(barbers::enabled.eq(true))
                .filter(barbers::merchant_id.eq(merchant_id))
                .filter(barbers::user_id.eq(login_info.user_id))
            ))
            .get_result(&mut *conn)
            .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"get_result error".to_string()))?;
            if existed_cellphone{
                return Err((StatusCode::BAD_REQUEST,"该商户已添加该手机号码的理发师".to_string())); //TODO 添加后验证email有效性
            }
        }
    }
    
    let mut user:Option<User>=None;

    if req.email.is_some(){
        let login_info=login_infos::table
            .filter(login_infos::enabled.eq(true))
            .filter(login_infos::login_info_type.eq("Email"))
            .filter(login_infos::login_info_account.eq(req.email.clone().unwrap()))
            .get_result::<LoginInfo>(&mut *conn)
            .ok();
        if let Some(login_info)=login_info{
            let res=users::table
            .filter(users::user_id.eq(login_info.user_id))
            .filter(users::enabled.eq(true))
            .get_result(&mut *conn).map_err(|e|{
                tracing::error!("{}",e.to_string());
                (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
            })?;
            user=Some(res);
        } else {
            if user.is_none(){
                let user_id=Uuid::new_v4();
                let new_user=NewUser{
                    user_id: &user_id,
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
                .get_result(&mut *conn).map_err(|e|{
                    tracing::error!("{}",e.to_string());
                    (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
                })?;
                user=Some(res);
            }
            
            let login_info=NewLoginInfo{
                login_info_id: &Uuid::new_v4(),
                login_info_account: &req.email.clone().unwrap(),
                login_info_type: "Email", //TODO get enum variant value string
                user_id: &user.clone().unwrap().user_id,
                enabled: true, 
                create_time: Local::now(),
                update_time: Local::now(),
            };
            diesel::insert_into(login_infos::table)
            .values(&login_info)
            .execute(&mut *conn).map_err(|e|{
                tracing::error!("{}",e.to_string());
                (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
            })?;
        }
    } 

    if req.cellphone.is_some(){
        let login_info=login_infos::table
            .filter(login_infos::enabled.eq(true))
            .filter(login_infos::login_info_type.eq("Cellphone"))
            .filter(login_infos::login_info_account.eq(req.cellphone.clone().unwrap()))
            .get_result::<LoginInfo>(&mut *conn)
            .ok();
        if let Some(login_info)=login_info{
            let res=users::table
            .filter(users::user_id.eq(login_info.user_id))
            .filter(users::enabled.eq(true))
            .get_result(&mut *conn).map_err(|e|{
                tracing::error!("{}",e.to_string());
                (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
            })?;
            user=Some(res);
            
        } else {
            if user.is_none(){
                let user_id=Uuid::new_v4();
                let new_user=NewUser{
                    user_id: &user_id,
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
                .get_result(&mut *conn).map_err(|e|{
                    tracing::error!("{}",e.to_string());
                    (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
                })?;
                user=Some(res);
            }
            
            let login_info=NewLoginInfo{
                login_info_id: &Uuid::new_v4(),
                login_info_account: &req.cellphone.clone().unwrap(),
                login_info_type: "Cellphone", //TODO get enum variant value string
                user_id: &user.clone().unwrap().user_id,
                enabled: true, 
                create_time: Local::now(),
                update_time: Local::now(),
            };
            diesel::insert_into(login_infos::table)
            .values(&login_info)
            .execute(&mut *conn).map_err(|e|{
                tracing::error!("{}",e.to_string());
                (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
            })?;
        }
    } 

    let new_barber=NewBarber{
        user_id:  &user.clone().unwrap().user_id,
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
    .get_result(&mut *conn).map_err(|e|{
        tracing::error!("{}",e.to_string());
        (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
    })?;

    // TODO 有密码需添加password_login_providers

    // add permissions
    for &permission_code in authorization_policy::DEFAULT_PERMISSIONS_OF_MERCHANT_BARBER{
        let permissions=permissions::table
            .filter(permissions::permission_id.eq_any(serde_json::from_str::<Vec<Uuid>>(&user.clone().unwrap().permissions).unwrap())) 
            .filter(permissions::enabled.eq(true))
            .get_results::<Permission>(&mut *conn)
            .unwrap();
        let mut permission_ids=permissions.iter().map(|p|p.permission_id).collect::<Vec<_>>();
        if !permissions.into_iter().any(|p|p.permission_code==permission_code){
            let permission_id=permissions::table
            .filter(permissions::permission_code.eq(permission_code)) 
            .filter(permissions::enabled.eq(true))
            .select(permissions::permission_id)
            .get_result::<Uuid>(&mut *conn)
            .unwrap();

            permission_ids.push(permission_id);

            diesel::update(
                users::table
                .filter(users::user_id.eq(user.clone().unwrap().user_id))
                .filter(users::enabled.eq(true))
            )
            .set((
                    users::permissions.eq(serde_json::to_string(&permission_ids).unwrap()),
                    users::update_time.eq(Local::now())
                ))
            .execute(&mut *conn).map_err(|e|{
                tracing::error!("{}",e.to_string());
                (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
            })?;
        }
    }

    let merchant=merchants::table
    .filter(merchants::enabled.eq(true))
    .filter(merchants::merchant_id.eq(merchant_id))
    .get_result(&mut *conn).map_err(|e|{
        tracing::error!("{}",e.to_string());
        (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
    })?;

    let barber_response=BarberResponse{barber,merchant};

    Ok(Json(barber_response))
}

pub async fn delete_barber(
    State(pool):State<AxumPgPool>,
    Path(barber_id):Path<Uuid>, 
    auth: AuthSession<AxumPgPool, AxumPgPool,User>,
)->Result<(),(StatusCode,String)>{
    //检查登录
    let _=auth.identity.as_ref().ok_or((StatusCode::UNAUTHORIZED,"no login".to_string()))?;

    //检查权限
    auth.require_permissions(vec![authorization_policy::MERCHANT_ADMINISTRATOR])
        .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"no permission".to_string()))?;
    
    let mut conn=pool.pool.get().unwrap();//TODO error

    let count=diesel::update(
        barbers::table
        .filter(barbers::barber_id.eq(barber_id))
        .filter(barbers::enabled.eq(true))
    )
    .set((
        barbers::enabled.eq(false),
        barbers::update_time.eq(Local::now())
    ))
    .execute(&mut *conn).map_err(|e|{
        tracing::error!("{}",e.to_string());
        (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
    })?;

    if count!=1 {
        return Err((StatusCode::NOT_FOUND,"data not exists".to_string()));
    }

    Ok(())
}

#[derive(Deserialize)]
pub struct BarberEditRequest{
    pub cellphone:Option<String>,
    pub real_name:Option<String>,
    pub email:Option<String>,
    pub permission_codes:Vec<String>,
}

pub async fn update_barber(
    State(pool):State<AxumPgPool>,
    Path(barber_id):Path<Uuid>, 
    auth: AuthSession<AxumPgPool, AxumPgPool,User>,
    Json(req): Json<BarberEditRequest>
)->Result<(),(StatusCode,String)>{
    //检查登录
    let _=auth.identity.as_ref().ok_or((StatusCode::UNAUTHORIZED,"no login".to_string()))?;

    //检查权限
    auth.require_permissions(vec![authorization_policy::MERCHANT_ADMINISTRATOR])
        .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"no permission".to_string()))?;

    let mut conn=pool.pool.get().unwrap();//TODO error
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
        .map_err(|e|{
            tracing::error!("{}",e.to_string());
            (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
        })?;

    if req.email.is_some(){
        let email_login_info=login_infos::table
            .filter(login_infos::enabled.eq(true))
            .filter(login_infos::login_info_type.eq("Email"))
            .filter(login_infos::login_info_account.eq(req.email.clone().unwrap()))
            .get_result::<LoginInfo>(&mut *conn)
            .ok();
        if let Some(login_info)=email_login_info{
            if login_info.user_id!=barber.user_id{
                return Err((StatusCode::BAD_REQUEST,"该邮箱已被其他用户使用".to_string())); //TODO 添加后验证email有效性
            }
        }
    }

    if req.cellphone.is_some(){
        let cellphone_login_info=login_infos::table
        .filter(login_infos::enabled.eq(true))
        .filter(login_infos::login_info_type.eq("Cellphone"))
        .filter(login_infos::login_info_account.eq(req.cellphone.clone().unwrap()))
        .get_result::<LoginInfo>(&mut *conn)
        .ok();
        if let Some(login_info)=cellphone_login_info{
            if login_info.user_id!=barber.user_id{
                return Err((StatusCode::BAD_REQUEST,"该手机号已被其他用户使用".to_string())); //TODO 添加后验证email有效性
            }
        }
    }

    if req.email.is_some(){
        let login_info=login_infos::table
            .filter(login_infos::enabled.eq(true))
            .filter(login_infos::login_info_type.eq("Email"))
            .filter(login_infos::login_info_account.eq(req.email.clone().unwrap()))
            .get_result::<LoginInfo>(&mut *conn)
            .ok();
        if login_info.is_none(){
            let login_info=NewLoginInfo{
                login_info_id: &Uuid::new_v4(),
                login_info_account: &req.email.clone().unwrap(),
                login_info_type: "Email", //TODO get enum variant value string
                user_id: &barber.user_id,
                enabled: true, 
                create_time: Local::now(),
                update_time: Local::now(),
            };
            diesel::insert_into(login_infos::table)
            .values(&login_info)
            .execute(&mut *conn).map_err(|e|{
                tracing::error!("{}",e.to_string());
                (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
            })?;
        }
    } 

    if req.cellphone.is_some(){
        let login_info=login_infos::table
            .filter(login_infos::enabled.eq(true))
            .filter(login_infos::login_info_type.eq("Cellphone"))
            .filter(login_infos::login_info_account.eq(req.cellphone.clone().unwrap()))
            .get_result::<LoginInfo>(&mut *conn)
            .ok();
        if login_info.is_none(){
            let login_info=NewLoginInfo{
                login_info_id: &Uuid::new_v4(),
                login_info_account: &req.cellphone.clone().unwrap(),
                login_info_type: "Cellphone", //TODO get enum variant value string
                user_id: &barber.user_id,
                enabled: true, 
                create_time: Local::now(),
                update_time: Local::now(),
            };
            diesel::insert_into(login_infos::table)
            .values(&login_info)
            .execute(&mut *conn).map_err(|e|{
                tracing::error!("{}",e.to_string());
                (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
            })?;
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
    .execute(&mut *conn).map_err(|e|{
        tracing::error!("{}",e.to_string());
        (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
    })?;

    // update permissions
    for permission_code in req.permission_codes{
        let permissions=permissions::table
            .filter(permissions::permission_id.eq_any(serde_json::from_str::<Vec<Uuid>>(&user.permissions).unwrap())) 
            .filter(permissions::enabled.eq(true))
            .get_results::<Permission>(&mut *conn)
            .unwrap();
        let mut permission_ids=permissions.iter().map(|p|p.permission_id).collect::<Vec<_>>();
        if !permissions.into_iter().any(|p|p.permission_code==permission_code){
            let permission_id=permissions::table
            .filter(permissions::permission_code.eq(permission_code)) 
            .filter(permissions::enabled.eq(true))
            .select(permissions::permission_id)
            .get_result::<Uuid>(&mut *conn)
            .unwrap();

            permission_ids.push(permission_id);

            diesel::update(
                users::table
                .filter(users::user_id.eq(user.user_id))
                .filter(users::enabled.eq(true))
            )
            .set((
                    users::permissions.eq(serde_json::to_string(&permission_ids).unwrap()),
                    users::update_time.eq(Local::now())
                ))
            .execute(&mut *conn).map_err(|e|{
                tracing::error!("{}",e.to_string());
                (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
            })?;
        }
    }
    
    Ok(())
}

#[derive(Serialize)]
pub struct BarberEditResponse{
    #[serde(flatten)]
    pub barber:Barber,
    pub permission_codes:Vec<String>,
}

pub async fn get_barber(
    State(pool):State<AxumPgPool>,
    Path(barber_id):Path<Uuid>, 
    auth: AuthSession<AxumPgPool, AxumPgPool,User>,
)->Result<Json<BarberEditResponse>,(StatusCode,String)>{
    //检查登录
    let _=auth.identity.as_ref().ok_or((StatusCode::UNAUTHORIZED,"no login".to_string()))?;

    //检查权限
    auth.require_permissions(vec![authorization_policy::MERCHANT_ADMINISTRATOR])
        .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"no permission".to_string()))?;

    let mut conn=pool.pool.get().unwrap();//TODO error  
    let merchant_id=serde_json::from_str::<Uuid>(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();

    let (barber,user)=barbers::table.inner_join(users::table.on(barbers::user_id.eq(users::user_id)))
    .filter(users::enabled.eq(true))
    .filter(barbers::barber_id.eq(barber_id))
    .filter(barbers::merchant_id.eq(merchant_id))
    .filter(barbers::enabled.eq(true))
    .get_result::<(Barber,User)>(&mut *conn)
    .map_err(|e|{
        tracing::error!("{}",e.to_string());
        (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
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
