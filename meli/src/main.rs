use std::{net::SocketAddr, sync::{Arc, Mutex}, str::FromStr};

use axum::{Router, routing::{get, post}, http::{Method, header}};
use axum_database_sessions::{ AxumSessionStore, AxumSessionLayer,AxumSessionConfig};
use axum_sessions_auth::{AuthSessionLayer, AxumAuthConfig};
use tower_http::{trace::TraceLayer, cors::Any};
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt,util::SubscriberInitExt};

use meli_backend::{ axum_pg_pool::AxumPgPool, models::User, utils::get_connection, handlers::{user_handler::*}};
use uuid::Uuid;

#[tokio::main]
async fn main(){
    let axum_pg_pool=AxumPgPool{
        connection:Arc::new(Mutex::new(get_connection()))
    };
    
    let session_config = AxumSessionConfig::default()
        .with_cookie_domain("http://127.0.0.1:8080")
        .with_cookie_same_site(cookie::SameSite::None)
        .with_http_only(false); //TODO key life_span cookie_name  memory_lifespan->zero
    let auth_config = AxumAuthConfig::<Uuid>::default(); //TODO with anoymous user id   auth_cookie_name
    let session_store = AxumSessionStore::<AxumPgPool>::new(Some(axum_pg_pool.clone()), session_config);

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or("meli_backend=trace".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app=Router::new()
        .route("/login", post(login_by_username))
        .route("/logout", get(logout))
        .route("/identity", get(get_current_identity))
        .route("/consumers", get(get_consumers).post(add_consumer))
        .route("/consumers/:c_id", get(get_consumer).post(update_consumer).delete(delete_consumer))
        .layer(CorsLayer::new()
            .allow_origin(["http://127.0.0.1:8080".parse().unwrap()])
            .allow_methods([axum::http::Method::GET,axum::http::Method::POST,axum::http::Method::DELETE,axum::http::Method::OPTIONS])
            .allow_headers([
                axum::http::header::CONTENT_TYPE,
                axum::http::header::ORIGIN,
                axum::http::header::ACCEPT,
                axum::http::header::SET_COOKIE,
                axum::http::header::COOKIE,
                header::HeaderName::from_str("credentials").unwrap()
                ])
                // Origin, Content-Type, Accept, Authorization, X-Request-With, Set-Cookie, Cookie, Bearer');
            .allow_credentials(true))
        .layer(AuthSessionLayer::<User, Uuid, AxumPgPool, AxumPgPool>::new(Some(axum_pg_pool.clone())).with_config(auth_config))
        .layer(AxumSessionLayer::new(session_store))
        .layer(TraceLayer::new_for_http());

    let addr=SocketAddr::from(([127,0,0,1],3000));
    tracing::debug!("listening on {}",addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
