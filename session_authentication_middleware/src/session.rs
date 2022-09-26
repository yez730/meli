use async_trait::async_trait;
use axum_core::extract::FromRequestParts;
use axum_session_middleware::{database_pool::AxumDatabasePool, session::AxumSession, constants::session_keys};
use http::{self, request::Parts, StatusCode};
use uuid::Uuid;
use std::{fmt::{self, Debug}, marker::PhantomData, sync::{Arc, Mutex}};

use crate::user::Identity;

#[derive(Debug, Clone)]
pub struct AuthSession<SessionP,AuthP,User>
where
    User:Authentication<User,AuthP> + Clone + Send + Sync + 'static,
    AuthP: Clone + Send + Sync + fmt::Debug + 'static,
    SessionP: AxumDatabasePool + Clone + fmt::Debug + Sync + Send + 'static,
{
    pub identity: Option<Identity>,
    pub axum_session: Arc<Mutex<AxumSession<SessionP>>>,
    pub database_pool:AuthP,
    pub phantom_user: PhantomData<User>,
}

#[async_trait]
pub trait Authentication<User,AuthP>
where
    User:Authentication<User,AuthP> + Clone + Send + Sync + 'static,
    AuthP: Clone + Send + Sync + fmt::Debug + 'static,
{
    fn load_identity(user_id:Uuid,pool:AuthP) -> Identity;
}

impl<SessionP,AuthP,User> AuthSession<SessionP,AuthP,User>
where
    User:Authentication<User,AuthP> + Clone + Send + Sync +Debug,
    AuthP: Clone + Send + Sync + fmt::Debug + 'static,
    SessionP: AxumDatabasePool + Clone + fmt::Debug + Sync + Send + 'static,
{
    pub fn require_permissions(&self,perms:Vec<&str>)->Result<(),&str>{
        match self.identity {
            Some(ref identity)=>{
                let permission_ok=perms.into_iter()
                    .all(|p|identity.permission_codes.iter().map(|p|p.as_str()).collect::<Vec<_>>().contains(&p)); //TODO extend predication
                
                if permission_ok{
                    Ok(())
                } else {
                    Err("no permissions.")
                }
            }
            None=>Err("no login."),
        }
    }

    pub async fn sign_in(&mut self,user_id:Uuid){
        {
            let mut session=self.axum_session.lock().unwrap();
            if session.get_user_id().is_some(){
                session.clear(); 
            }
            session.set_user_id(user_id);
        }

        self.refresh_identity().await;
    }

    // 新user_id 或 变更权限 时， 需要refresh_identity
    pub async fn refresh_identity(&mut self){
        let mut session=self.axum_session.lock().unwrap();

        if let Some(user_id) =session.get_user_id(){
            let identity=User::load_identity(user_id,self.database_pool.clone());

            session.set_data(session_keys::IDENTITY.to_string(), serde_json::to_string(&identity).unwrap());
        }
    }

    pub async fn sign_out(&mut self){
        self.axum_session.lock().unwrap().clear();
    }
}

#[async_trait]
impl<S, SessionP,AuthP,User> FromRequestParts<S> for AuthSession<SessionP,AuthP,User>
where
    User:Authentication<User,AuthP> + Clone + Send + Sync + 'static,
    AuthP: Clone + Send + Sync + fmt::Debug + 'static,
    SessionP: AxumDatabasePool + Clone + fmt::Debug + Sync + Send + 'static,
    S: Send + Sync,
{
    type Rejection = (http::StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthSession<SessionP,AuthP,User>>()
            .cloned()
            .ok_or((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Can't extract AuthSession. Is `AuthSessionLayer` enabled?",
            ))
    }
}
