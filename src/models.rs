use chrono::Local;
use diesel::prelude::*;
use uuid::Uuid;

use crate::schema::permissions;

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
    pub user_id: Uuid,
    pub username: String,
    pub client_id: Uuid,
    pub client_ip: String,
    pub client_type: String,
    pub source_request_id: Uuid,
    pub init_time: chrono::DateTime<Local>,
    pub expiry_time: chrono::DateTime<Local>,
    pub create_time: chrono::DateTime<Local>,
    pub update_time: chrono::DateTime<Local>,
    pub extra: Option<String>,
}

#[derive(Queryable)]
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