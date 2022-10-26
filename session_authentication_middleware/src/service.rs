use axum_core::{
    body::{self, BoxBody},
    response::Response,
    BoxError,
};
use axum_session_middleware::{
    session::AxumSession, database::AxumDatabaseTrait, constants::session_keys,
};
use bytes::Bytes;
use futures::future::BoxFuture;
use http::{self, Request, StatusCode};
use http_body::{Body as HttpBody, Full};
use std::{
    boxed::Box,
    convert::Infallible,
    fmt,
    marker::PhantomData,
    task::{Context, Poll}, sync::{Mutex, Arc},
};
use tower_service::Service;

use crate::{ user::Identity,session::{AuthSession, Authentication}};

#[derive(Clone)]
pub struct AuthSessionService<S,AuthDB,User,SessionDB>
where
    AuthDB: Clone + Send + Sync + fmt::Debug + 'static,
    User:Authentication<User,AuthDB> + Clone + Send + Sync + 'static,
    SessionDB: AxumDatabaseTrait + Clone + fmt::Debug + Sync + Send + 'static,
{
    pub(crate) database:AuthDB,
    pub(crate) inner: S,
    pub phantom_user: PhantomData<User>,
    pub phantom_session_db: PhantomData<SessionDB>,
}

impl<S, ReqBody, ResBody,User,AuthDB,SessionDB> Service<Request<ReqBody>>
    for AuthSessionService<S,AuthDB,User,SessionDB>
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
    User:Authentication<User,AuthDB> + Clone + Send + Sync + 'static,
    AuthDB: Clone + Send + Sync + fmt::Debug + 'static,
    SessionDB: AxumDatabaseTrait + Clone + fmt::Debug + Sync + Send + 'static,
{
    type Response = Response<BoxBody>;
    type Error = Infallible;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<ReqBody>) -> Self::Future {
        let database=self.database.clone();
        let not_ready_inner = self.inner.clone();
        let mut ready_inner = std::mem::replace(&mut self.inner, not_ready_inner); //TODO 

        Box::pin(async move {
            let axum_session = match req.extensions().get::<Arc<Mutex<AxumSession<SessionDB>>>>().cloned() {
                Some(session) => session,
                None => {
                    return Ok(Response::builder()
                        .status(StatusCode::UNAUTHORIZED)
                        .body(body::boxed(Full::from("401 Unauthorized")))
                        .unwrap());
                }
            };

            let identity={
                let session=axum_session.lock().unwrap();
                if session.get_user_id().is_some() {
                    Some(serde_json::from_str::<Identity>(session.get_data(session_keys::IDENTITY)).unwrap())
                } else {
                    None
                }
            };
            
            let auth_session:AuthSession<SessionDB,AuthDB,User> = AuthSession {
                phantom_user:PhantomData::default(),
                identity,
                axum_session: axum_session,
                database:database.clone(),
            };
           
            req.extensions_mut().insert(auth_session);

            Ok(ready_inner.call(req).await?.map(body::boxed))
        })
    }
}

impl<S, AuthDB,User,SessionDB> fmt::Debug for AuthSessionService<S,AuthDB,User,SessionDB>
where
    S: fmt::Debug,
    User:Authentication<User,AuthDB> + Clone + Send + Sync + 'static,
    AuthDB: Clone + Send + Sync + fmt::Debug + 'static,
    SessionDB: AxumDatabaseTrait + Clone + fmt::Debug + Sync + Send + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthSessionService")
            .field("inner", &self.inner)
            .finish()
    }
}
