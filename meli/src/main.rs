use std::{net::{SocketAddr, IpAddr}, str::FromStr};
use axum::{Router, routing::{get, post}, http::{HeaderValue, header, Method}};
use axum_session_authentication_middleware::layer::AuthSessionLayer;
use axum_session_middleware::{layer::AxumSessionLayer, session_store::AxumSessionStore, config::AxumSessionConfig};

use tower_http::{trace::TraceLayer};
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt,util::SubscriberInitExt};

use dotenvy::dotenv;

use meli_backend::{ axum_pg::AxumPg, models::User, utils::get_connection_pool, handlers::*};

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
    dotenv().expect("Cannot find .env file.");
    
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").expect("Cannot find RUST_LOG environment variable."),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let axum_pg=AxumPg{
        pool:get_connection_pool()
    };

    let env_var=std::env::var("MELI").unwrap_or("DEV".into());
    let (host_ip,frontend_port,backend_port)=if env_var=="PROD" {
        (
            std::env::var("PROD_HOST_IP").expect("Cannot find HOST_IP environment variable."),
            std::env::var("PROD_FRONTEND_PORT").expect("Cannot find FRONTEND_PORT environment variable.").parse::<u16>().expect("Not available BACKEND_PORT value"),
            std::env::var("PROD_BACKEND_PORT").expect("Cannot find BACKEND_PORT environment variable.").parse::<u16>().expect("Not available BACKEND_PORT value"),
        )
    } else {
        (
            std::env::var("DEV_HOST_IP").expect("Cannot find HOST_IP environment variable."),
            std::env::var("DEV_FRONTEND_PORT").expect("Cannot find FRONTEND_PORT environment variable.").parse::<u16>().expect("Not available BACKEND_PORT value"),
            std::env::var("DEV_BACKEND_PORT").expect("Cannot find BACKEND_PORT environment variable.").parse::<u16>().expect("Not available BACKEND_PORT value"),
        )
    };

    let cross_origin= if frontend_port ==80 {
        format!("http://{}",host_ip.as_str())
    } else {
        format!("http://{}:{}",host_ip.as_str(),frontend_port)
    };

    let app=Router::with_state(axum_pg.clone())
        .route("/login", post(barber_login_by_password))
        .route("/identity/logout", get(logout))
        .route("/identity/current", get(get_current_identity))

        .route("/register/merchant", post(register_merchant))
        
        .route("/barber", get(get_current_barber).post(update_info))

        .route("/merchant/current", get(get_current_merchant))
        .route("/merchant/barbers", get(get_barbers).post(add_barber))
        .route("/merchant/barber/:barber_id", get(get_barber).post(update_barber).delete(delete_barber))

        .route("/merchant/get_all_permissions", get(get_all_permissions))

        .route("/members", get(get_members).post(add_member))
        .route("/member/:member_id", get(get_member).post(update_member).delete(delete_member))
        .route("/member/recharge/:member_id", post(recharge))

        .route("/member/orders/:member_id", get(get_orders_by_member_id))
        .route("/member/recharge_records/:member_id", get(get_recharge_records_by_member_id))

        .route("/service_types", get(get_service_types).post(add_service_type))
        .route("/service_type/:service_type_id", get(get_service_type).post(update_service_type).delete(delete_service_type))
        
        .route("/appointments",get(get_appointments).post(add_appointment))
        .route("/appointment/:appointment_id",get(get_appointment))

        .route("/statistic/orders",get(get_orders))
        .route("/statistic/recharge_records",get(get_recharge_records))

        .layer(CorsLayer::new()
            .allow_origin(cross_origin.parse::<HeaderValue>().unwrap(),)
            .allow_headers([
                header::CONTENT_TYPE,
                header::HeaderName::from_str("X-SID").unwrap(),
                ])
            .allow_methods([Method::GET,Method::POST,Method::DELETE])
            .allow_credentials(true)
        )
        .layer(AuthSessionLayer::<AxumPg, AxumPg,User>::new(axum_pg.clone()))
        .layer(AxumSessionLayer::new(
            AxumSessionStore::new(axum_pg.clone(),
            AxumSessionConfig::default().with_cookie_domain(host_ip.clone()))
        ))
        .layer(TraceLayer::new_for_http());

    let addr=SocketAddr::from((
        IpAddr::from_str(host_ip.as_str()).unwrap(),backend_port));

    tracing::debug!("listening on {}",addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
