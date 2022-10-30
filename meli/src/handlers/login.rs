use axum::{http::StatusCode, Json, extract::State};
use axum_session_authentication_middleware::session::AuthSession;
use serde::Deserialize;
use crate::{
    schema::*,
    models::{Barber, Merchant, LoginInfo, PasswordLoginProvider}
};
use diesel::prelude::*; 
use crate::{models::User, axum_pg::AxumPg};
use crate::constant;

use super::barber::BarberResponse;

#[derive(Deserialize)]
pub struct BarberLoginRequest{
    pub account:String, //cellphone or email

    pub password:String,
}
pub async fn barber_login_by_password(State(pg):State<AxumPg>,mut auth: AuthSession<AxumPg, AxumPg,User>,Json(req):Json<BarberLoginRequest>)->Result<Json<BarberResponse>,(StatusCode,String)>{
    let mut conn=pg.pool.get().unwrap();

    let login_info=login_infos::table
        .filter(login_infos::login_info_account.eq(req.account))
        .filter(login_infos::enabled.eq(true))
        .get_result::<LoginInfo>(&mut *conn)
        .ok();

    if login_info.is_none(){
        return Err((StatusCode::UNAUTHORIZED,"用户未注册".into()));
    }

    let provider=password_login_providers::table
        .filter(password_login_providers::user_id.eq(login_info.as_ref().unwrap().user_id))
        .filter(password_login_providers::enabled.eq(true))
        .get_result::<PasswordLoginProvider>(&mut *conn)
        .ok();
    if provider.is_none(){
        return Err((StatusCode::BAD_REQUEST,"用户禁止登录".into()));
    }

    argon2::verify_encoded(&provider.unwrap().password_hash, req.password.as_bytes())
        .map_err(|_|(StatusCode::BAD_REQUEST,"密码验证失败".to_string()))?;

    let barber_response=barbers::table
        .inner_join(merchants::table.on(barbers::merchant_id.eq(merchants::merchant_id)))
        .filter(barbers::enabled.eq(true))
        .filter(barbers::user_id.eq(login_info.as_ref().unwrap().user_id))
        .filter(merchants::enabled.eq(true))
        .get_result::<(Barber,Merchant)>(&mut *conn)
        .map(|bm|BarberResponse{ barber:bm.0,merchant:bm.1})
        .ok();
    if barber_response.is_none(){
        return Err((StatusCode::BAD_REQUEST,"商户登录失败".into()));
    }

    let merchant_id=barber_response.as_ref().unwrap().merchant.merchant_id;

    auth.sign_in(login_info.as_ref().unwrap().user_id).await;
    auth.axum_session.lock().unwrap().set_data(constant::MERCHANT_ID.to_owned(), merchant_id.to_string());
    
    Ok(Json(barber_response.unwrap()))
}
