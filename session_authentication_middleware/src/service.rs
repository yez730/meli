use axum_core::{
    body::{self, BoxBody},
    response::Response,
    BoxError,
};
use axum_session_middleware::{
    database_pool::AxumDatabasePool,
    session::AxumSession,
};
use bytes::Bytes;
use chrono::Utc;
use futures::future::BoxFuture;
use http::{self, Request, StatusCode};
use http_body::{Body as HttpBody, Full};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    boxed::Box,
    convert::Infallible,
    fmt,
    hash::Hash,
    marker::PhantomData,
    task::{Context, Poll},
};
use tower_service::Service;

use crate::{ user::Identity, Authentication,session::AuthSession};


#[derive(Clone)]
pub struct AuthSessionService<S,P,User>
where
    P: AxumDatabasePool + Clone + fmt::Debug + Sync + Send + 'static,
    User:Authentication<User>,
{
    pub(crate) inner: S,
    pub phantom_session: PhantomData<P>,
    pub phantom_user: PhantomData<User>,
}

impl<S, ReqBody, ResBody,User,P> Service<Request<ReqBody>>
    for AuthSessionService<S,P,User>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>, Error = Infallible>
        + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
    ReqBody: Send + 'static,
    Infallible: From<<S as Service<Request<ReqBody>>>::Error>,
    ResBody: HttpBody<Data = Bytes> + Send + 'static,
    ResBody::Error: Into<BoxError>,
    User:Authentication<User> + Clone + Send + Sync + 'static,
    P: AxumDatabasePool + Clone + fmt::Debug + Sync + Send + 'static,
{
    type Response = Response<BoxBody>;
    type Error = Infallible;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<ReqBody>) -> Self::Future {
        let not_ready_inner = self.inner.clone();
        let mut ready_inner = std::mem::replace(&mut self.inner, not_ready_inner);

        Box::pin(async move {
            let axum_session = match req.extensions().get::<AxumSession<P>>().cloned() { //TODO P需要限定？ PhantomData？
                Some(session) => session,
                None => {
                    return Ok(Response::builder()
                        .status(StatusCode::UNAUTHORIZED)
                        .body(body::boxed(Full::from("401 Unauthorized")))
                        .unwrap());
                }
            };

            let (i,u)=if let Some(user_id)=axum_session.get_logined_user_id(){
                match serde_json::from_str::<Identity>(axum_session.get_identity_str()) {
                    Ok(identity)=>{
                        let user=User::get_user(user_id).await;
                        (Some(identity),Some(user))
                    }
                    Err(e)=>{
                        tracing::error!("get identity error: {}",e); //TODO .....
                        (None,None)
                    }
                }
            } else{
                (None,None)
            };

            let auth_session = AuthSession {
                user:u,
                authenticatied_identity:i,
                axum_session: axum_session,
            };

            // Sets a clone of the Store in the Extensions for Direct usage and sets the Session for Direct usage
            req.extensions_mut().insert(auth_session.clone());

            Ok(ready_inner.call(req).await?.map(body::boxed))
        })
    }
}

impl<S, P,User> fmt::Debug for AuthSessionService<S,P,User>
where
    S: fmt::Debug,
    User: Authentication<User> + fmt::Debug + Clone + Send,
    P: AxumDatabasePool + Clone + fmt::Debug + Sync + Send + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthSessionService")
            .field("inner", &self.inner)
            .finish()
    }
}
