use axum_core::{
    body::{self, BoxBody},
    response::Response,
    BoxError,
};
use bytes::Bytes;
use cookie::{Cookie, CookieJar, Key};
use futures::future::BoxFuture;
use http::{
    self,
    header::{COOKIE, SET_COOKIE},
    HeaderMap, Request, HeaderValue,
};
use http_body::Body as HttpBody;
use uuid::Uuid;
use std::{
    boxed::Box,
    convert::Infallible,
    fmt::{self, Debug, Formatter},
    marker::{Send, Sync},
    task::{Context, Poll},
};
use tower_service::Service;
use chrono::{Local, DateTime};

use crate::{database_pool::AxumDatabasePool, session_store::AxumSessionStore, session::{AxumSession, SessionId}, session_data::AxumSessionData, constants::{SESSIONID, USERID}};

#[derive(Clone)]
pub struct AxumSessionService<S, T>
where
    T: AxumDatabasePool + Clone + Debug + Sync + Send + 'static,
{
    pub(crate) session_store: AxumSessionStore<T>,
    pub(crate) inner: S,
}

impl<S, T, ReqBody, ResBody> Service<Request<ReqBody>> for AxumSessionService<S, T>
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
    T: AxumDatabasePool + Clone + Debug + Sync + Send + 'static,
{
    type Response = Response<BoxBody>;
    type Error = Infallible;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<ReqBody>) -> Self::Future {
        let store = self.session_store.clone();
        let not_ready_inner = self.inner.clone();
        let mut ready_inner = std::mem::replace(&mut self.inner, not_ready_inner);

        Box::pin(async move {
            let session_id=req.headers().get(SESSIONID).and_then(|id|id.to_str().ok());
           
            let mut session = AxumSession::load_or_init(&store, session_id).await.unwrap_or_else(|e|{
                tracing::error!("load_or_init error: {}",e);

                let session_id=SessionId::init_session_id();
                
                AxumSession{
                    session_id:session_id.clone(),
                    store:store.clone(),
                    session_data:AxumSessionData::init(session_id.get_session_guid(), store.config.memory_clear_timeout),
                    is_modified:false,
                }
            });

            store.memory_store.retain(|_k, v| v.init_time + store.config.memory_clear_timeout > Local::now() && v.user_id.is_none() || v.expiry_time>Local::now());

            req.extensions_mut().insert(session.clone());

            let mut response = ready_inner.call(req).await?.map(body::boxed);

            let _=session.commit().await.map_err(|e|{
                tracing::error!("session commit error: {}",e);
            });

            response.headers_mut().insert(SESSIONID, HeaderValue::from_str(&session.session_id.0).unwrap());

            Ok(response)
        })
    }
}

impl<S, T> Debug for AxumSessionService<S, T>
where
    S: Debug,
    T: AxumDatabasePool + Clone + Debug + Sync + Send + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("AxumSessionService")
            .field("session_store", &self.session_store)
            .field("inner", &self.inner)
            .finish()
    }
}