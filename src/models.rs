use anyhow::{anyhow,Error};
use async_trait::async_trait;
use axum_sessions_auth::{HasPermission, Authentication};
use chrono::Local;
use diesel::prelude::*;
use uuid::Uuid;

use crate::{schema::*, axum_pg_pool::AxumPgPool};

#[derive(Queryable)]
pub struct Permission{
    pub id: i64,
    pub permission_id: Uuid,
    pub permission_code: String,
    pub permission_name :String,
    pub description: String,
    pub create_time: chrono::DateTime<Local>,
    pub update_time: chrono::DateTime<Local>,
    pub data: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name=permissions)]
pub struct NewPermission<'a>{
    pub permission_id: &'a Uuid,
    pub permission_code: &'a str,
    pub permission_name :&'a str,
    pub description: &'a str,
    pub create_time: chrono::DateTime<Local>,
    pub update_time: chrono::DateTime<Local>,
    pub data: Option<&'a str>,
}

#[derive(Queryable)]
pub struct Session{
    pub id: i64,
    pub session_id: Uuid,
    pub data: String,
    pub expiry_time: chrono::DateTime<Local>,
    pub create_time: chrono::DateTime<Local>,
    pub update_time: chrono::DateTime<Local>,
}

#[derive(Insertable)]
#[diesel(table_name=sessions)]
pub struct NewSession<'a> {
    pub session_id: &'a Uuid,
    pub data: &'a str,
    pub expiry_time: chrono::DateTime<Local>,
    pub create_time: chrono::DateTime<Local>,
    pub update_time: chrono::DateTime<Local>,
}

#[derive(Queryable,Clone, Debug)]
pub struct User{
    pub id: i64,
    pub user_id: Uuid,
    pub username: String,
    pub description: String,
    pub permissions: String,
    pub roles: String,
    pub create_time: chrono::DateTime<Local>,
    pub update_time: chrono::DateTime<Local>,
    pub data: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name=users)]
pub struct NewUser<'a>{
    pub user_id: &'a Uuid,
    pub username: &'a str,
    pub description: &'a str,
    pub permissions: &'a str,
    pub roles: &'a str,
    pub create_time: chrono::DateTime<Local>,
    pub update_time: chrono::DateTime<Local>,
    pub data: Option<&'a str>,
}


// This is only used if you want to use Token based Authentication checks
#[async_trait]
impl HasPermission<AxumPgPool> for User {
    async fn has(&self, perm: &str, _pool: &Option<&AxumPgPool>) -> bool {
        let rights:Vec<&str>=serde_json::from_str(&self.permissions).unwrap();

        rights.contains(&&perm)
    }
}

#[async_trait]
impl Authentication<User, Uuid, AxumPgPool> for User {
    async fn load_user(userid: Uuid, pool: Option<&AxumPgPool>) -> Result<User, Error> {
        use crate::schema::{users::dsl::*};

        let mut conn=pool.unwrap().connection.lock().map_err(|e|anyhow!(e.to_string()))?;

        users
            .filter(user_id.eq(userid))
            .get_result::<User>(&mut *conn)
            .map_err(|e|anyhow!(e.to_string()))
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
pub struct Consumer{
    pub id: i64,
    pub user_id: Uuid,
    pub consumer_id: Uuid,
    pub cellphone:String,
    pub email:Option<String>,
    pub credential_no:Option<String>,
    pub real_name:Option<String>,
    pub create_time: chrono::DateTime<Local>,
    pub update_time: chrono::DateTime<Local>,
    pub data: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name=consumers)]
pub struct NewConsumer<'a>{
    pub user_id: &'a Uuid,
    pub consumer_id: &'a Uuid,
    pub cellphone:&'a str,
    pub email:Option<&'a str>,
    pub credential_no:Option<&'a str>,
    pub real_name:Option<&'a str>,
    pub create_time: chrono::DateTime<Local>,
    pub update_time: chrono::DateTime<Local>,
    pub data: Option<&'a str>,
}


#[derive(Queryable)]
pub struct Role{
    pub id: i64,
    pub role_id: Uuid,
    pub role_code: String,
    pub role_name:String,
    pub permissions:String,
    pub description:String,
    pub create_time: chrono::DateTime<Local>,
    pub update_time: chrono::DateTime<Local>,
    pub data: Option<String>,
}

#[derive(Queryable)]
pub struct Account{
    pub id: i64,
    pub user_id: Uuid,
    pub account_id: Uuid,
    pub merchant_id: Uuid,
    pub cellphone:String,
    pub email:Option<String>,
    pub credential_no:Option<String>,
    pub real_name:Option<String>,
    pub create_time: chrono::DateTime<Local>,
    pub update_time: chrono::DateTime<Local>,
    pub data: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name=accounts)]
pub struct NewAccount<'a>{
    pub user_id: &'a Uuid,
    pub account_id: &'a Uuid,
    pub merchant_id: &'a Uuid,
    pub cellphone:&'a str,
    pub email:Option<&'a str>,
    pub credential_no:Option<&'a str>,
    pub real_name:Option<&'a str>,
    pub create_time: chrono::DateTime<Local>,
    pub update_time: chrono::DateTime<Local>,
    pub data: Option<&'a str>,
}


#[derive(Queryable)]
pub struct Merchant{
    pub id: i64,
    pub merchant_id: Uuid,
    pub merchant_name:String,
    pub company_name:Option<String>,
    pub credential_no:Option<String>,
    pub create_time: chrono::DateTime<Local>,
    pub update_time: chrono::DateTime<Local>,
    pub data: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name=merchants)]
pub struct NewMerchant<'a>{
    pub merchant_id: &'a Uuid,
    pub merchant_name:&'a str,
    pub company_name:Option<&'a str>,
    pub credential_no:Option<&'a str>,
    pub create_time: chrono::DateTime<Local>,
    pub update_time: chrono::DateTime<Local>,
    pub data: Option<&'a str>,
}


#[derive(Queryable)]
pub struct LoginInfo{
    pub id: i64,
    pub login_info_id: Uuid,
    pub login_info_account: String,
    pub login_info_type: String,
    pub user_id: Uuid,
    pub enabled: bool,
    pub create_time: chrono::DateTime<Local>,
}

#[derive(Insertable)]
#[diesel(table_name=login_infos)]
pub struct NewLoginInfo<'a>{
    pub login_info_id: &'a Uuid,
    pub login_info_account: &'a str,
    pub login_info_type: &'a str,
    pub user_id: &'a Uuid,
    pub enabled: bool,
    pub create_time: chrono::DateTime<Local>,
}

#[derive(Queryable)]
pub struct PasswordLoginProvider{
    pub id: i64,
    pub user_id: Uuid,
    pub password_hash: String,
    pub create_time: chrono::DateTime<Local>,
    pub update_time: chrono::DateTime<Local>,
    pub data:Option<String>
}

#[derive(Insertable)]
#[diesel(table_name=password_login_providers)]
pub struct NewPasswordLoginProvider<'a>{
    pub user_id: &'a Uuid,
    pub password_hash: &'a str,
    pub create_time: chrono::DateTime<Local>,
    pub update_time: chrono::DateTime<Local>,
    pub data:Option<&'a str>
}
