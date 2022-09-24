use async_trait::async_trait;
use axum_core::extract::FromRequestParts;
use axum_session_middleware::{database_pool::AxumDatabasePool, session::AxumSession, constants::SessionKeys};
use http::{self, request::Parts, StatusCode};
use uuid::Uuid;
use std::fmt;

use crate::user::Identity;

#[derive(Debug, Clone)]
pub struct AuthSession<P,User>
where
    User:Authentication<User>,
    P: AxumDatabasePool + Clone + fmt::Debug + Sync + Send + 'static,
{
    pub user:Option<User>,
    pub authenticatied_identity: Option<Identity>,
    pub axum_session: AxumSession<P>,
}

#[async_trait]
pub trait Authentication<User>
where
{
    async fn get_user(user_id:Uuid)->User;
    async fn load_identity(&self) -> Identity;
}

#[async_trait]
impl<S, P,User> FromRequestParts<S> for AuthSession<P,User>
where
    P: AxumDatabasePool + Clone + fmt::Debug + Sync + Send + 'static,
    User:Authentication<User> + Clone + Send + Sync + 'static,
    S: Send + Sync,
{
    type Rejection = (http::StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthSession<P,User>>()
            .cloned()
            .ok_or((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Can't extract AuthSession. Is `AuthSessionLayer` enabled?",
            ))
    }
}

impl<P,User> AuthSession<P,User>
where
    User:Authentication<User>,
    P: AxumDatabasePool + Clone + fmt::Debug + Sync + Send + 'static,
{
    pub fn require_permissions(&self,perms:Vec<&str>)->Result<(),&str>{
        match self.authenticatied_identity {
            Some(ref identity)=>{
                let permission_ok=perms.into_iter().all(|p|identity.PermissionCodes.iter().map(|p|p.as_str()).collect::<Vec<_>>().contains(&p)); //TODO extend predication
                if permission_ok{
                    Ok(())
                } else {
                    Err("no permissions.")
                }
            }
            None=>Err("no login."),
        }
    }

    pub async fn sign_in(&mut self,user_id:Uuid){ //TODO &mut 
        if self.axum_session.get_logined_user_id().is_some(){
            self.axum_session.clear(); 
        }
        self.axum_session.set_user_id(user_id);
        self.refresh_identity().await;
    }

    //TODO 新user_id / 权限变更
     pub async fn refresh_identity(&mut self){
        if let Some(ref user) =self.user{
            let identity_str=serde_json::to_string(&user.load_identity().await);
            if let Ok(identity_str)=identity_str{
                self.axum_session.set_data(SessionKeys::Identity.to_string(), identity_str); //TODO SessionKeys::Identity.to_string() ??????
            }
        }
    }

    pub fn sign_out(&mut self){
        self.axum_session.clear();
    }
}
