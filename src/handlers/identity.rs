use axum::{http::StatusCode, Json};
use axum_session_authentication_middleware::session::AuthSession;

use crate::{models::User, axum_pg::AxumPg};
use axum_session_authentication_middleware::user::Identity;

pub async fn logout(mut auth: AuthSession< AxumPg, AxumPg,User>){
    auth.sign_out().await;
}

pub async fn get_current_identity(auth: AuthSession<AxumPg, AxumPg,User>)->Result<Json<Identity>,(StatusCode,String)>{
    let identity=auth.identity.ok_or((StatusCode::UNAUTHORIZED,"No login".to_string()))?;
    Ok(Json(identity))
}
