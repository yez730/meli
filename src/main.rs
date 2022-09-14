use std::{net::SocketAddr, sync::{Arc, Mutex}};

use async_trait::async_trait;
use axum::{Router, routing::get };
use axum_database_sessions::{ AxumSessionStore, AxumSessionLayer,AxumSessionConfig};
use axum_sessions_auth::{AuthSession, AuthSessionLayer, Authentication, AxumAuthConfig, HasPermission};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt,util::SubscriberInitExt};

use meli_backend::{*, axum_pg_pool::AxumPgPool, models::User};
use uuid::Uuid;

fn test<S>(s:S)
where 
    S: Send+Sync+'static{
    
}

#[tokio::main]
async fn main(){
    let conn = establish_connection();
    let axum_pg_pool=AxumPgPool{
        connection:Arc::new(Mutex::new(conn))
    };

    // test(axum_pg_pool.clone());
    // let s:AuthSession<User, Uuid, AxumPgPool, AxumPgPool>=Default::default();

    let session_config = AxumSessionConfig::default(); //key life_span cookie_name

    let auth_config = AxumAuthConfig::<Uuid>::default(); //TODO with anoymous user id   auth_cookie_name
    let session_store = AxumSessionStore::<AxumPgPool>::new(Some(axum_pg_pool.clone()), session_config);


    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or("meli_backend=trace".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app=Router::new()
        .route("/login", get(login))
        .route("/loginout", get(loginout))
        .route("/index", get(index))
        .layer(TraceLayer::new_for_http())
        .layer(AxumSessionLayer::new(session_store))
        .layer(AuthSessionLayer::<User, Uuid, AxumPgPool, AxumPgPool>::new(Some(axum_pg_pool.clone())).with_config(auth_config));

    let addr=SocketAddr::from(([127,0,0,1],3000));
    tracing::debug!("listening on {}",addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn login()->&'static str{
    "Hello, World!"
}

async fn loginout()->&'static str{
    "Ok!"
}

async fn index(auth: AuthSession<User, Uuid, AxumPgPool, AxumPgPool>)->&'static str{
    "Ok!"
}

