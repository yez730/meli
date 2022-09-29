use axum_core::{
    body::{self, BoxBody},
    response::Response,
    BoxError,
};
use bytes::Bytes;
use cookie::{CookieJar, Cookie, Key};
use futures::future::BoxFuture;
use http::{
    self,Request, HeaderValue, header, HeaderMap,
};
use http_body::Body as HttpBody;
use std::{
    boxed::Box,
    convert::Infallible,
    fmt::{self, Debug, Formatter},
    marker::{Send, Sync},
    task::{Context, Poll}, sync::{Mutex, Arc},
};
use tower_service::Service;
use chrono::{Local, Duration};

use crate::{database_pool::AxumDatabasePool, session_store::AxumSessionStore, session::AxumSession, constants::SESSIONID, config::AxumSessionConfig};

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
            let session_id= get_metadata(&req,SESSIONID,store.config.clone());
            let session = AxumSession::load_or_init(&store, session_id.as_deref()).await;
            store.memory_store.retain(|_k, v|  v.expiry_time>Local::now());

            let session=Arc::new(Mutex::new(session)); // 在res返回中取不到同一个Extension

            req.extensions_mut().insert(Arc::clone(&session));

            let mut res = ready_inner.call(req).await?.map(body::boxed);

            let session=Arc::clone(&session);
            
            let mut session=session.lock().unwrap().to_owned();
            let _=session.commit().await.map_err(|e|{
                tracing::error!("session commit error: {}",e);
            });

            update_metadata(&mut res,SESSIONID,session.session_id.0.as_str(),store.config.clone());

            Ok(res)
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

fn get_metadata<ReqBody>(req:&Request<ReqBody>,name:&str,config:AxumSessionConfig) ->Option<String>{
    let mut value=req.headers().get(name)
        .and_then(|id|id.to_str().ok())
        .map(|x|x.to_string());

    if value.is_none(){
        let cookies=get_cookies(req);

        value=cookies.get_cookie(name, &config.key)
        .map(|c| c.value().to_string());
    }

    value
}

fn update_metadata(res:&mut Response<BoxBody>,name:&'static str,value:&str,config:AxumSessionConfig){
    res.headers_mut().insert(name, HeaderValue::from_str(value).unwrap()).unwrap();

    let mut cookies = CookieJar::new();

    cookies.add_cookie(
        create_cookie(config.clone(), name.to_string(), value.to_string()),
        &config.key,
    );

    set_cookies(cookies, res.headers_mut());
}

pub(crate) trait CookiesExt {
    fn get_cookie(&self, name: &str, key: &Option<Key>) -> Option<Cookie<'static>>;
    fn add_cookie(&mut self, cookie: Cookie<'static>, key: &Option<Key>);
}

impl CookiesExt for CookieJar {
    fn get_cookie(&self, name: &str, key: &Option<Key>) -> Option<Cookie<'static>> {
        if let Some(key) = key {
            self.private(key).get(name)
        } else {
            self.get(name).cloned()
        }
    }

    fn add_cookie(&mut self, cookie: Cookie<'static>, key: &Option<Key>) {
        if let Some(key) = key {
            self.private_mut(key).add(cookie)
        } else {
            self.add(cookie)
        }
    }
}

fn create_cookie<'a>(
    config: AxumSessionConfig,
    name:String,
    value: String,
) -> Cookie<'a> {
    let mut cookie_builder = Cookie::build(name, value)
        .path(config.cookie_path.to_owned())
        .secure(config.cookie_secure)
        .http_only(config.cookie_http_only)
        .same_site(config.cookie_same_site);

    if let Some(domain) = &config.cookie_domain {
        cookie_builder = cookie_builder.domain(domain.to_owned());
    }

    let time_duration = Duration::max_value().to_std().expect("Max Age out of bounds");
    cookie_builder = cookie_builder.max_age(time_duration.try_into().expect("Max Age out of bounds"));

    cookie_builder.finish()
}

fn get_cookies<ReqBody>(req: &Request<ReqBody>) -> CookieJar {
    let mut jar = CookieJar::new();

    let cookie_iter = req
        .headers()
        .get_all(header::COOKIE)
        .into_iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|cookie| Cookie::parse_encoded(cookie.to_owned()).ok());

    for cookie in cookie_iter {
        jar.add_original(cookie);
    }

    jar
}

fn set_cookies(jar: CookieJar, headers: &mut HeaderMap) {
    for cookie in jar.delta() {
        if let Ok(header_value) = cookie.encoded().to_string().parse() {
            headers.append(header::SET_COOKIE, header_value);
        }
    }
}
