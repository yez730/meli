pub mod models;
pub mod schema;
pub mod axum_pg_pool;

use chrono::Local;
use models::*;
use uuid::Uuid;
use std::env;

use diesel::PgConnection;
use diesel::prelude::*;
use dotenvy::dotenv;

pub fn establish_connection()->PgConnection{
    dotenv().ok();

    let database_url=env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .unwrap_or_else(|_|panic!("Error connecting to {}",database_url))

}

pub fn show_permissions(conn:&mut PgConnection){
    use crate::schema::permissions::dsl::*;

    let connection=&mut establish_connection();

    let result=permissions
        .filter(is_enabled.eq(true))
        .limit(5)
        .load::<Permission>(connection)
        .expect("Error loading permissions");

    println!("Displaying {} permissions",result.len());

    for permission in result{
        println!("{}----{}",permission.permission_id,permission.create_time);
    }
}

pub fn create_permission(conn:&mut PgConnection)->Permission{
    use crate::schema::permissions;

    let new_permission=NewPermission{
        permission_id: Uuid::new_v4(),
        permission_code: "Token::Index",
        permission_name :"Token Index",
        description: "Token Index",
        is_enabled: false,
        create_time: Local::now(),
        update_time: Local::now(),
        extra: None,
    };

    diesel::insert_into(permissions::table)
        .values(&new_permission)
        .get_result(conn)
        .expect("Error saving permission")
}

pub fn create_user(conn:&mut PgConnection)->User{
    use crate::schema::users;

    let new_user=NewUser{
        user_id: Uuid::new_v4(),
        username: "yez",
        description: "",
        is_enabled: true,
        roles: "[\"Admin\"]",
        permissions: "[\"Token::Index\"]",
        create_time: Local::now(),
        update_time: Local::now(),
        extra: None,
    };

    diesel::insert_into(users::table)
        .values(&new_user)
        .get_result(conn)
        .expect("Error saving user")
}


pub fn update_permission_enabled(conn:&mut PgConnection,id:i32){
    use self::schema::permissions::dsl::{permissions, is_enabled};

    let permission=diesel::update(permissions.find(id))
        .set(is_enabled.eq(true))
        .get_result::<Permission>(conn)
        .unwrap();

    println!("Enabled permission name `{}`",permission.permission_name);
}


/*
 let connection = &mut establish_connection();
    let num_deleted = diesel::delete(posts.filter(title.like(pattern)))
        .execute(connection)
        .expect("Error deleting posts");
*/
#[cfg(test)]
mod test{
use super::*;

    #[test]
    fn test_create_permission(){
        let connection = &mut establish_connection();
        create_permission(connection);
        assert!(true);
    }

    #[test]
    fn test_create_user(){
        let connection = &mut establish_connection();
        create_user(connection);
        assert!(true);
    }
}