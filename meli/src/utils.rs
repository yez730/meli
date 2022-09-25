
use std::env;

use diesel::PgConnection;
use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use diesel::r2d2::Pool;
use dotenvy::dotenv;

// TODO  use diesel::PgConnection pool.?   // add to axum state??
// pub fn get_connection()->PgConnection{
//     dotenv().ok();

//     let database_url=env::var("DATABASE_URL").expect("DATABASE_URL must be set");
//     PgConnection::establish(&database_url)
//         .unwrap_or_else(|_|panic!("Error connecting to {}",database_url))
// }

pub fn get_connection_pool()->Pool<ConnectionManager<PgConnection>> {
    dotenv().ok();

    let database_url=env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    Pool::builder()
        .test_on_check_out(true)
        .build(manager)
        .expect("Could not build connection pool")
}