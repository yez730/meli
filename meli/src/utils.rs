
use std::env;

use diesel::PgConnection;
use diesel::r2d2::ConnectionManager;
use diesel::r2d2::Pool;
use dotenvy::dotenv;

pub fn get_connection_pool()->Pool<ConnectionManager<PgConnection>> {
    dotenv().expect("Cannot find .env file.");
    let database_url=env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    Pool::builder()
        .test_on_check_out(true)
        .build(manager)
        .expect("Could not build connection pool")
}