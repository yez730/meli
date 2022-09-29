use std::{net::SocketAddr, str::FromStr};

use axum::{Router, routing::{get, post}, http::{HeaderValue, header, Method}};
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
        .route("/consumer/:c_id", get(get_consumer).post(update_consumer).delete(delete_consumer))

        
        .layer(CorsLayer::new()
            .allow_origin("http://192.168.8.108:8080".parse::<HeaderValue>().unwrap(),)
            .allow_headers([
                header::CONTENT_TYPE,
                header::HeaderName::from_str("X-SID").unwrap(),
                ])
            .allow_methods([Method::GET,Method::POST,Method::DELETE])
            .expose_headers([header::HeaderName::from_str("X-SID").unwrap(),]) //TODO delete when using only cookie auth?
            .allow_credentials(true)
        )
      
        // .route_layer(axum::middleware::from_fn(temporay_add_cors_header)) // 

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

// Access-Control-Allow-Private-Network true // TODO delete
// async fn temporay_add_cors_header<B>(req: axum::http::Request<B>, next: axum::middleware::Next<B>) -> Result<axum::response::Response, axum::http::StatusCode> {
//     let (parts, body) = req.into_parts();

//     if parts.method == Method::OPTIONS{
//         let mut res=next.run(axum::http::Request::from_parts(parts, body)).await;
//         res.headers_mut().insert("Access-Control-Allow-Private-Network", HeaderValue::from_str("true").unwrap());
//         Ok(res)
//     } else {
//         let res=next.run(axum::http::Request::from_parts(parts, body)).await;
//         Ok(res)
//     }
// }
