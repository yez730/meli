use std::{net::SocketAddr};

use axum::{Router, routing::{get, post}};
use axum_session_authentication_middleware::layer::AuthSessionLayer;
use axum_session_middleware::{layer::AxumSessionLayer, session_store::AxumSessionStore};
use tower_http::{trace::TraceLayer, cors::Any};
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt,util::SubscriberInitExt};

use meli_backend::{ axum_pg_pool::AxumPgPool, models::User, utils::get_connection_pool, handlers::{user_handler::*}};

#[tokio::main]
async fn main(){
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or("meli_backend=trace,axum_session_authentication_middleware=trace,axum_session_middleware=trace".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let axum_pg_pool=AxumPgPool{
        pool:get_connection_pool()
    };

    let app=Router::with_state(axum_pg_pool.clone())
        .route("/login", post(login_by_username))
        .route("/logout", get(logout))
        .route("/identity", get(get_current_identity))
        .route("/consumers", get(get_consumers).post(add_consumer))
        .route("/consumers/:c_id", get(get_consumer).post(update_consumer).delete(delete_consumer))
        .layer(CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
            .allow_credentials(false))
        .layer(AuthSessionLayer::<AxumPgPool, AxumPgPool,User>::new(axum_pg_pool.clone()))
        .layer(AxumSessionLayer::new(AxumSessionStore::new(axum_pg_pool.clone())))
        .layer(TraceLayer::new_for_http());

    let addr=SocketAddr::from(([127,0,0,1],3000));
    tracing::debug!("listening on {}",addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
