use std::net::SocketAddr;

use axum::{Router, routing::get };
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt,util::SubscriberInitExt};

use meli_backend::*;

#[tokio::main]
async fn main(){
    // let conn=&mut establish_connection();

    // create_permission(conn);
    // println!("Created permission");

    // show_permissions(conn);

    // update_permission_enabled(conn,1);
    // return;

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or("meli_backend=trace".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app=Router::new()
        .route("/", get(handler))
        .layer(TraceLayer::new_for_http());

    let addr=SocketAddr::from(([127,0,0,1],3000));
    tracing::debug!("listening on {}",addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn handler()->&'static str{
    "Hello, World!"
}