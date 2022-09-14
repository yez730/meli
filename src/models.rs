use anyhow::{anyhow,Error};
use async_trait::async_trait;
use axum_database_sessions::SessionError;
use axum_sessions_auth::{HasPermission, Authentication};
use chrono::Local;
use diesel::prelude::*;
use uuid::Uuid;

use crate::{schema::*, axum_pg_pool::AxumPgPool};

#[derive(Queryable)]
pub struct Permission{
    pub id: i32,
    pub permission_id: Uuid,
    pub permission_code: String,
    pub permission_name :String,
    pub description: String,
    pub is_enabled: bool,
    pub create_time: chrono::DateTime<Local>,
    pub update_time: chrono::DateTime<Local>,
    pub extra: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name=permissions)]
pub struct NewPermission<'a>{
    pub permission_id: Uuid,
    pub permission_code: &'a str,
    pub permission_name :&'a str,
    pub description: &'a str,
    pub is_enabled: bool,
    pub create_time: chrono::DateTime<Local>,
    pub update_time: chrono::DateTime<Local>,
    pub extra: Option<&'a str>,
}

#[derive(Queryable)]
pub struct Role{
    pub id: i32,
    pub role_id: Uuid,
    pub role_code: String,
    pub role_name: String,
    pub description: String,
    pub is_enabled: bool,
    pub permissions: String,
    pub create_time: chrono::DateTime<Local>,
    pub update_time: chrono::DateTime<Local>,
    pub extra: Option<String>,
}

#[derive(Queryable)]
pub struct Session{
    pub id: i64,
    pub session_id: Uuid,
    pub expiry_time: chrono::DateTime<Local>,
    pub extra: String,
}

#[derive(Insertable)]
#[diesel(table_name=sessions)]
pub struct NewSession<'a> {
    pub session_id: Uuid,
    pub expiry_time: chrono::DateTime<Local>,
    pub extra: &'a str,
}

#[derive(Queryable,Clone, Debug)]
pub struct User{
    pub id: i64,
    pub user_id: Uuid,
    pub username: String,
    pub description: String,
    pub is_enabled: bool,
    pub roles: String,
    pub permissions: String,
    pub create_time: chrono::DateTime<Local>,
    pub update_time: chrono::DateTime<Local>,
    pub extra: Option<String>,
}

// This is only used if you want to use Token based Authentication checks
#[async_trait]
impl HasPermission<AxumPgPool> for User {
    async fn has(&self, perm: &str, pool: &Option<&AxumPgPool>) -> bool {
        let rights:Vec<&str>=serde_json::from_str(&self.permissions).unwrap(); //TODO unwrap ok?
        if rights.contains(&&perm) {
            true
        }else {
            false
        }
    }
}

#[async_trait]
impl Authentication<User, Uuid, AxumPgPool> for User {
    async fn load_user(userid: Uuid, pool: Option<&AxumPgPool>) -> Result<User, Error> {
        use crate::schema::{users::dsl::*};

        let mut conn=pool.unwrap().connection.lock().map_err(|e|anyhow!("Get connection error"))?;

        let db_users=users
            .filter(user_id.eq(userid))
            .limit(1)
            .load::<User>(&mut *conn)?;

        if db_users.len()==0 {
            return Err(anyhow!("No user found"));
        }

        Ok(db_users[0].clone())
    }

    fn is_authenticated(&self) -> bool {
        true
    }

    fn is_active(&self) -> bool {
        true
    }

    fn is_anonymous(&self) -> bool {
        false
    }
}


#[derive(Queryable)]
pub struct Account{
    pub id: i64,
    pub user_id: Uuid,
    pub account_id:Uuid,
    pub merchant_id:Uuid,
    pub cellphone:String,
    pub account_name:String,
    pub account_state:String,
    pub account_photo_url:Option<String>,
    pub credential_type:Option<String>,
    pub credential_no:Option<String>,
    pub real_name:Option<String>,
    pub emp_no:Option<String>,
    pub email:Option<String>,
    pub is_enabled:bool,
    pub create_time: chrono::DateTime<Local>,
    pub update_time: chrono::DateTime<Local>,
    pub extra: Option<String>,
}

#[derive(Queryable)]
pub struct Consumer{
    pub id: i64,
    pub user_id: Uuid,
    pub consumer_id: Uuid,
    pub cellphone:String,
    pub real_name:Option<String>,
    pub credential_type:Option<String>,
    pub credential_no:Option<String>,
    pub selfie_photo_url:Option<String>,
    pub is_verified:bool,
    pub two_factor_verify:bool,
    pub is_enabled:bool,
    pub create_time: chrono::DateTime<Local>,
    pub update_time: chrono::DateTime<Local>,
    pub extra: Option<String>,
}

#[derive(Queryable)]
pub struct Merchant{
    pub id: i64,
    pub merchant_id: Uuid,
    pub merchant_name:String,
    pub is_enabled:bool,
    pub create_time: chrono::DateTime<Local>,
    pub update_time: chrono::DateTime<Local>,
    pub extra: Option<String>,
}