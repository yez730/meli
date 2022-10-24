use std::{net::SocketAddr, str::FromStr};
use axum::{Router, routing::{get, post}, http::{HeaderValue, header, Method}};
use axum_session_authentication_middleware::layer::AuthSessionLayer;
use axum_session_middleware::{layer::AxumSessionLayer, session_store::AxumSessionStore, config::AxumSessionConfig};

use tower_http::{trace::TraceLayer};
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt,util::SubscriberInitExt};

use dotenvy::dotenv;

use meli_backend::{ axum_pg_pool::AxumPgPool, models::User, utils::get_connection_pool, handlers::*};

use appointment::*;
use barber::*;
use identity::*;
use login::*;
use member::*;
use merchant::*;
use register::*;
use service_type::*;
use statistic::*;

#[tokio::main]
async fn main(){
    dotenv().expect("Cannot find .env file");
    
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
        .route("/login", post(barber_login_by_password))
        .route("/identity/logout", get(logout))
        .route("/identity/current", get(get_current_identity))

        .route("/register/merchant", get(register_merchant))
        
        .route("/barber", get(get_current_barber).post(update_info))

        .route("/merchant/all_merchants_by_account", get(get_merchants_by_login_account))

        .route("/merchant/current", get(get_current_merchant))
        .route("/merchant/barbers", get(get_barbers).post(add_barber))
        .route("/merchant/barber/:barber_id", get(get_barber).post(update_barber).delete(delete_barber))

        .route("/members", get(get_members).post(add_member))
        .route("/member/:member_id", get(get_member).post(update_member).delete(delete_member))
        .route("/member/recharge/:member_id", post(recharge))

        .route("/service_types", get(get_service_types).post(add_service_type))
        .route("/service_type/:service_type_id", get(get_service_type).post(update_service_type).delete(delete_service_type))
        
        .route("/appointments",get(get_appointments).post(add_appointment))
        .route("/appointment/:appointment_id",get(get_appointment))

        .route("/statistic/orders",get(get_orders))
        .route("/statistic/recharge_records",get(get_recharge_records))

        .layer(CorsLayer::new()
            .allow_origin("http://127.0.0.1:8080".parse::<HeaderValue>().unwrap(),)
            .allow_headers([
                header::CONTENT_TYPE,
                header::HeaderName::from_str("X-SID").unwrap(),
                ])
            .allow_methods([Method::GET,Method::POST,Method::DELETE])
            .allow_credentials(true)
        )
        .layer(AuthSessionLayer::<AxumPgPool, AxumPgPool,User>::new(axum_pg_pool.clone()))
        .layer(AxumSessionLayer::new(
            AxumSessionStore::new(axum_pg_pool.clone(),
            AxumSessionConfig::default().with_cookie_domain("127.0.0.1"))
        ))
        .layer(TraceLayer::new_for_http());

    let addr=SocketAddr::from(([127,0,0,1],3000));
    tracing::debug!("listening on {}",addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
