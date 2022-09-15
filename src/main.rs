use std::{net::SocketAddr, sync::{Arc, Mutex}};

use async_trait::async_trait;
use axum::{Router, routing::get, http::Method };
use axum_database_sessions::{ AxumSessionStore, AxumSessionLayer,AxumSessionConfig};
use axum_sessions_auth::{AuthSession, AuthSessionLayer, Authentication, AxumAuthConfig, HasPermission, Auth, Rights};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt,util::SubscriberInitExt};

use meli_backend::{*, axum_pg_pool::AxumPgPool, models::User};
use uuid::Uuid;

#[tokio::main]
async fn main(){
    let conn = establish_connection();
    let axum_pg_pool=AxumPgPool{
        connection:Arc::new(Mutex::new(conn))
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
        .route("/login", get(login))
        .route("/loginout", get(loginout))
        .route("/index", get(index))
        
        .layer(AuthSessionLayer::<User, Uuid, AxumPgPool, AxumPgPool>::new(Some(axum_pg_pool.clone())).with_config(auth_config))
        .layer(AxumSessionLayer::new(session_store))
        .layer(TraceLayer::new_for_http())
        ;

    let addr=SocketAddr::from(([127,0,0,1],3000));
    tracing::debug!("listening on {}",addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn login(auth: AuthSession<User, Uuid, AxumPgPool, AxumPgPool>)->&'static str{
    use crate::schema::{users::dsl::*};
    use diesel::prelude::*; // for .filter
    let mut conn = establish_connection();

    //1. check user ok
    let us=users
            .filter(username.eq("yez"))
            .limit(1)
            .load::<User>(&mut conn).unwrap();
    let user=us[0].clone();

    //2. set user to coolie
    tracing::debug!("user.user_id {}",user.user_id);
    auth.login_user(user.user_id).await;

    "login ok"
    /*
    let username = if !auth.is_authenticated() {
            // Set the user ID of the User to the Session so it can be Auto Loaded the next load or redirect
            auth.login_user(2);
            "".to_string()
        } else {
            // If the user is loaded and is Authenticated then we can use it.
            if let Some(user) = auth.current_user {
                user.username.clone()
            } else {
                "".to_string()
            }
        };

        format!("{}-{}", username, count)[..]
    } else {
        return format!("please login first").as_str();
        // if !auth.is_authenticated() {
        //     // Set the user ID of the User to the Session so it can be Auto Loaded the next load or redirect
        //     auth.login_user(2);
        //     // Set the session to be long term. Good for Remember me type instances.
        //     auth.remember_user(true);
        //     // Redirect here after login if we did indeed login.
        // }
    */

}

async fn loginout(auth: AuthSession<User, Uuid, AxumPgPool, AxumPgPool>)->&'static str{
    use crate::schema::{users::dsl::*};
    use diesel::prelude::*; // for .filter
    let mut conn = establish_connection();

    //1. check user ok
    let us=users
            .filter(username.eq("yez"))
            .limit(1)
            .load::<User>(&mut conn).unwrap();
    let user=us[0].clone();

    //2. log out user
    auth.logout_user().await;

    "log out ok"
}

async fn index(method: Method, auth: AuthSession<User, Uuid, AxumPgPool, AxumPgPool>)->String{
    let mut count: usize = auth.session.get("count").await.unwrap_or(0);
    count += 1;

    // Session is Also included with Auth so no need to require it in the function arguments if your using
    // AuthSession.
    auth.session.set("count", count).await;

    if let Some(cur_user) = auth.current_user {

        if !Auth::<User, Uuid, AxumPgPool>::build([Method::GET], false) //TODO auth_required ?
            .requires(Rights::any([
                Rights::permission("Token::Index")
            ]))
            .validate(&cur_user, &method, None)
            .await
        {
            return "No Permissions!".to_string();
        }

        return count.to_string();
    } else {
        return "no loged in".to_string()
    }
    
}
