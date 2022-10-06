use axum::{http::StatusCode, Json, extract::State};
use axum_session_authentication_middleware::session::AuthSession;
use serde::{Deserialize, Serialize};
use crate::{
    schema::*,
    models::{Barber, Merchant, Member}
};
use diesel::{
    prelude::*, // for .filter
}; 
use crate::{models::User, axum_pg_pool::AxumPgPool, login_manager::{get_login_info, password_login::verify_password}};
use axum_session_authentication_middleware::{ user as auth_user,session::Authentication};

#[derive(Deserialize)]
pub struct LoginRequest{
    pub username:String,
    pub password:String,
}

#[derive(Serialize)]
pub struct LoginResponse{
    pub identity:auth_user::Identity,
    pub barber :Option<BarberResonse>,
    pub member:Option<Member>
}

#[derive(Serialize)]
pub struct BarberResonse{
    #[serde(flatten)]
    pub barber :Barber,
    pub merchant:Merchant
}

pub async fn login_by_username(State(pool):State<AxumPgPool>,mut auth: AuthSession<AxumPgPool, AxumPgPool,User>,Json(req):Json<LoginRequest>)->Result<Json<LoginResponse>,(StatusCode,String)>{
    let mut conn=pool.pool.get().unwrap();//TODO error

    let login_info=get_login_info(req.username,&mut *conn).map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;
    
    verify_password(login_info.user_id, req.password,&mut *conn).map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;

    let user=users::dsl::users
            .filter(users::dsl::user_id.eq(login_info.user_id))
            .filter(users::dsl::enabled.eq(true))
            .get_result::<User>(&mut *conn).map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;

    let barber_merchant=barbers::table
        .inner_join(merchants::table.on(barbers::merchant_id.eq(merchants::merchant_id)))
        .filter(barbers::dsl::user_id.eq(user.user_id))
        .filter(barbers::dsl::enabled.eq(true))
        .get_result::<(Barber,Merchant)>(&mut *conn)
        .ok();

    let barber_response=barber_merchant.clone().map(|bm| BarberResonse{
        barber:bm.0,
        merchant:bm.1,
    });

    let member=members::dsl::members
        .filter(members::dsl::user_id.eq(user.user_id))
        .filter(members::dsl::enabled.eq(true))
        .get_result::<Member>(&mut *conn)
        .ok();
    
    let response=LoginResponse{
        identity:User::load_identity(login_info.user_id,pool.clone()),
        barber:barber_response,
        member,
    };
    auth.sign_in(login_info.user_id).await;
    auth.axum_session.lock().unwrap().set_data(String::from("barber"), serde_json::to_string(&barber_merchant.map(|bm|bm.0)).unwrap());
    
    Ok(Json(response))
}

pub async fn logout(mut auth: AuthSession< AxumPgPool, AxumPgPool,User>){
    auth.sign_out().await;
}

pub async fn get_current_identity(auth: AuthSession<AxumPgPool, AxumPgPool,User>)->Result<Json<auth_user::Identity>,(StatusCode,String)>{
    let identity=auth.identity.ok_or((StatusCode::UNAUTHORIZED,"no login".to_string()))?;
    Ok(Json(identity))
}
