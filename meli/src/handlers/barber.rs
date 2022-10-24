use std::env;

use axum::{http::StatusCode, Json, extract::State};
use axum_session_authentication_middleware::session::AuthSession;
use chrono::Local;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::{
    schema::*,
    models::{Barber,Merchant, LoginInfo, NewLoginInfo}, authorization_policy
};
use diesel::{
    prelude::*, // for .filter
    select, 
    dsl::exists,
};
use crate::constant; 
use crate::{models::User, axum_pg_pool::AxumPgPool};

#[derive(Serialize)]
pub struct BarberResponse{
    #[serde(flatten)]
    pub barber :Barber,
    pub merchant:Merchant
}

pub async fn get_current_barber(
    State(pool):State<AxumPgPool>,
    auth: AuthSession<AxumPgPool, AxumPgPool,User>,
)->Result<Json<BarberResponse>,(StatusCode,String)>{
    //检查登录
    let _=auth.identity.as_ref().ok_or((StatusCode::UNAUTHORIZED,"no login".to_string()))?;

    //检查权限
    auth.require_permissions(vec![authorization_policy::BARBER_BASE])
        .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"no permission".to_string()))?;

    let mut conn=pool.pool.get().unwrap();//TODO error  
    
    let merchant_id=serde_json::from_str::<Uuid>(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();
    let user_id=auth.identity.unwrap().user_id;

    let barber=barbers::table
        .inner_join(merchants::table.on(barbers::merchant_id.eq(merchants::merchant_id)))
        .filter(barbers::enabled.eq(true))
        .filter(barbers::merchant_id.eq(merchant_id))
        .filter(barbers::user_id.eq(user_id))
        .filter(merchants::enabled.eq(true))
        .get_result::<(Barber,Merchant)>(&mut *conn)
        .map(|bm|BarberResponse{ barber:bm.0,merchant:bm.1})
        .map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;
        
    Ok(Json(barber))
}

#[derive(Deserialize)]
pub struct UpdateInfoRequest{
    pub cellphone:Option<String>,
    pub real_name:Option<String>,
    pub email:Option<String>,

    #[serde(rename="newPassword")]
    pub new_password:Option<String>,

    pub old_password:Option<String>,

    #[serde(rename="merchantName")]
    pub merchant_name:String,
    #[serde(rename="merchantAdderss")]
    pub merchant_address:Option<String>,
    #[serde(rename="merchantRemark")]
    pub merchant_remark:Option<String>,
}

pub async fn update_info(
    State(pool):State<AxumPgPool>,
    auth: AuthSession<AxumPgPool, AxumPgPool,User>,
    Json(req): Json<UpdateInfoRequest>
)->Result<(),(StatusCode,String)>{
    //检查登录
    let _=auth.identity.as_ref().ok_or((StatusCode::UNAUTHORIZED,"no login".to_string()))?;

    //检查权限
    auth.require_permissions(vec![authorization_policy::BARBER_BASE])
        .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"no permission".to_string()))?;

    let mut conn=pool.pool.get().unwrap();//TODO error

    let merchant_id=serde_json::from_str::<Uuid>(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();
    let user_id=auth.identity.unwrap().user_id;

    if req.old_password.is_some() && req.new_password.is_none(){
        return Err((StatusCode::BAD_REQUEST,"新密码和旧密码不匹配".to_string()));
    }

    if req.email.is_some(){
        let login_info=login_infos::table
            .filter(login_infos::enabled.eq(true))
            .filter(login_infos::login_info_type.eq("Email"))
            .filter(login_infos::login_info_account.eq(req.email.clone().unwrap()))
            .get_result::<LoginInfo>(&mut *conn)
            .ok();
        if let Some(login_info)=login_info{
            if login_info.user_id!=user_id{
                return Err((StatusCode::BAD_REQUEST,"邮箱已被其他用户使用".to_string())); // TODO 提取单独方法，使用验证码总是可以修改
            }
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
            if login_info.user_id!=user_id{
                return Err((StatusCode::BAD_REQUEST,"手机号已被其他用户使用".to_string())); // TODO 提取单独方法，使用验证码总是可以修改
            }
        }
    }

    if req.old_password.is_some(){
        let salt = env::var("DATABASE_ENCRYPTION_SAULT").unwrap();
        let config = argon2::Config::default();

        let old_hash = argon2::hash_encoded(req.old_password.unwrap().as_bytes(), salt.as_bytes(), &config).unwrap();
        let is_old_password_match:bool=select(exists(
                password_login_providers::table
                .filter(password_login_providers::enabled.eq(true))
                .filter(password_login_providers::user_id.eq(user_id))
                .filter(password_login_providers::password_hash.eq(old_hash))
            ))
            .get_result(&mut *conn)
            .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"get_result error".to_string()))?;
        if !is_old_password_match{
            return Err((StatusCode::BAD_REQUEST,"旧密码不正确".to_string()));
        }

        let new_hash = argon2::hash_encoded(req.new_password.unwrap().as_bytes(), salt.as_bytes(), &config).unwrap();
        diesel::update(
            password_login_providers::table
            .filter(password_login_providers::user_id.eq(user_id))
            .filter(password_login_providers::enabled.eq(true))
        )
        .set((
                password_login_providers::password_hash.eq(new_hash),
                password_login_providers::update_time.eq(Local::now())
            ))
        .execute(&mut *conn).map_err(|e|{
            tracing::error!("{}",e.to_string());
            (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
        })?;
    }

    if req.email.is_some(){
        let email_login_info=login_infos::table
            .filter(login_infos::enabled.eq(true))
            .filter(login_infos::login_info_type.eq("Email"))
            .filter(login_infos::login_info_account.eq(req.email.clone().unwrap()))
            .get_result::<LoginInfo>(&mut *conn)
            .ok();
        if email_login_info.is_some(){
            diesel::update(
                login_infos::table
                .filter(login_infos::user_id.eq(user_id))
                .filter(login_infos::login_info_type.eq("Email"))
                .filter(login_infos::enabled.eq(true))
            )
            .set((
                login_infos::login_info_account.eq(req.email.clone().unwrap()),
                login_infos::update_time.eq(Local::now())
                ))
            .execute(&mut *conn).map_err(|e|{
                tracing::error!("{}",e.to_string());
                (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
            })?;
        }else {
            let login_info=NewLoginInfo{
                login_info_id: &Uuid::new_v4(),
                login_info_account: &req.email.clone().unwrap(),
                login_info_type: "Email",
                user_id: &user_id,
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
        let cellphone_login_info=login_infos::table
            .filter(login_infos::enabled.eq(true))
            .filter(login_infos::login_info_type.eq("Cellphone"))
            .filter(login_infos::login_info_account.eq(req.cellphone.clone().unwrap()))
            .get_result::<LoginInfo>(&mut *conn)
            .ok();
        if cellphone_login_info.is_some(){
            diesel::update(
                login_infos::table
                .filter(login_infos::user_id.eq(user_id))
                .filter(login_infos::login_info_type.eq("Cellphone"))
                .filter(login_infos::enabled.eq(true))
            )
            .set((
                login_infos::login_info_account.eq(req.cellphone.clone().unwrap()),
                login_infos::update_time.eq(Local::now())
                ))
            .execute(&mut *conn).map_err(|e|{
                tracing::error!("{}",e.to_string());
                (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
            })?;
        }else {
            let login_info=NewLoginInfo{
                login_info_id: &Uuid::new_v4(),
                login_info_account: &req.cellphone.clone().unwrap(),
                login_info_type: "Cellphone",
                user_id: &user_id,
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
        .filter(barbers::user_id.eq(user_id))
        .filter(barbers::merchant_id.eq(merchant_id))
        .filter(barbers::enabled.eq(true))
    )
    .set((
            barbers::cellphone.eq(req.cellphone.unwrap()), //TODO nullale
            barbers::real_name.eq(req.real_name), 
            barbers::email.eq(req.email),
            barbers::update_time.eq(Local::now())
        ))
    .execute(&mut *conn).map_err(|e|{
        tracing::error!("{}",e.to_string());
        (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
    })?;

    diesel::update(
        merchants::table
        .filter(merchants::merchant_id.eq(merchant_id))
        .filter(merchants::enabled.eq(true))
    )
    .set((
            merchants::merchant_name.eq(req.merchant_name),
            merchants::address.eq(req.merchant_address), 
            merchants::remark.eq(req.merchant_remark),
            merchants::update_time.eq(Local::now())
        ))
    .execute(&mut *conn).map_err(|e|{
        tracing::error!("{}",e.to_string());
        (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
    })?;
    
    Ok(())
}
