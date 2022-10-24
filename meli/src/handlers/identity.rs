use axum::{http::StatusCode, Json};
use axum_session_authentication_middleware::session::AuthSession;

use crate::{models::User, axum_pg_pool::AxumPgPool};
use axum_session_authentication_middleware::user::Identity;

pub async fn logout(mut auth: AuthSession< AxumPgPool, AxumPgPool,User>){
    auth.sign_out().await;
}

pub async fn get_current_identity(auth: AuthSession<AxumPgPool, AxumPgPool,User>)->Result<Json<Identity>,(StatusCode,String)>{
    let identity=auth.identity.ok_or((StatusCode::UNAUTHORIZED,"no login".to_string()))?;
    Ok(Json(identity))
}
