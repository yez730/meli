use axum::{http::StatusCode, Json, extract::State};
use axum_session_authentication_middleware::session::AuthSession;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::{
    schema::*,
    models::{Barber, Merchant, LoginInfo, PasswordLoginProvider}
};
use diesel::{
    prelude::*, // for .filter
}; 
use crate::{models::User, axum_pg_pool::AxumPgPool};
use axum_session_authentication_middleware::{ user as auth_user,session::Authentication};
use crate::constant;

use super::barber::BarberResponse;

#[derive(Deserialize)]
pub struct BarberLoginRequest{
    pub merchant_id:Uuid,
    pub account:String, //TODO cellphone or email  // use ripgrep
    pub password:String,
}
pub async fn barber_login_by_password(State(pool):State<AxumPgPool>,mut auth: AuthSession<AxumPgPool, AxumPgPool,User>,Json(req):Json<BarberLoginRequest>)->Result<Json<BarberResponse>,(StatusCode,String)>{
    let mut conn=pool.pool.get().unwrap();

    let login_info=login_infos::table
        .filter(login_infos::login_info_account.eq(req.account))
        .filter(login_infos::enabled.eq(true))
        .get_result::<LoginInfo>(&mut *conn)
        .map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;
    
    let provider=password_login_providers::table
        .filter(password_login_providers::user_id.eq(login_info.user_id))
        .filter(password_login_providers::enabled.eq(true))
        .get_result::<PasswordLoginProvider>(&mut *conn)
        .map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;
       
    argon2::verify_encoded(&provider.password_hash, req.password.as_bytes())
        .map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,"密码验证失败".to_string()))?;

    let barber=barbers::table
        .inner_join(merchants::table.on(barbers::merchant_id.eq(merchants::merchant_id)))
        .filter(barbers::enabled.eq(true))
        .filter(barbers::merchant_id.eq(req.merchant_id))
        .filter(barbers::user_id.eq(login_info.user_id))
        .filter(merchants::enabled.eq(true))
        .get_result::<(Barber,Merchant)>(&mut *conn)
        .map(|bm|BarberResponse{ barber:bm.0,merchant:bm.1})
        .map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;

    auth.sign_in(login_info.user_id).await;

    auth.axum_session.lock().unwrap().set_data(constant::MERCHANT_ID.to_owned(), barber.merchant.merchant_id.to_string());
    
    Ok(Json(barber))
}
