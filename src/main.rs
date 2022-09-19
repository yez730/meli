use std::{net::SocketAddr, sync::{Arc, Mutex}};

use axum::{Router, routing::{get, post}};
use axum_database_sessions::{ AxumSessionStore, AxumSessionLayer,AxumSessionConfig};
use axum_sessions_auth::{AuthSessionLayer, AxumAuthConfig};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt,util::SubscriberInitExt};

use meli_backend::{ axum_pg_pool::AxumPgPool, models::User, utils::get_connection, handlers::{user_handler::*}};
use uuid::Uuid;

#[tokio::main]
async fn main(){
    let axum_pg_pool=AxumPgPool{
        connection:Arc::new(Mutex::new(get_connection()))
    };
    
    let session_config = AxumSessionConfig::default(); //TODO key life_span cookie_name  memory_lifespan->zero

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
        .route("/consumers", get(get_consumers).post(add_consumer))
        .route("/consumers/:c_id", post(update_consumer).delete(delete_consumer))
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
