use anyhow::{anyhow,Error};
use async_trait::async_trait;
use axum_session_authentication_middleware::{ user as auth_user,session::Authentication};
use axum_session_middleware::database_pool::AxumDatabasePool;
use chrono::{Local, NaiveDate};
use diesel::{prelude::*, data_types::Cents};
use serde::Serialize;
use uuid::Uuid;

use crate::{schema::*, axum_pg_pool::AxumPgPool, my_date_format};

#[derive(Queryable,Clone, Debug)]
pub struct User{
    pub id: i64,

    pub user_id: Uuid,
    pub description: String,
    pub permissions: String,
    pub roles: String,
    pub enabled:bool,

    pub create_time: chrono::DateTime<Local>,
    pub update_time: chrono::DateTime<Local>,

    pub data: Option<String>,
}

#[derive(Queryable)]
pub struct Permission{
    pub id: i64,

    pub permission_id: Uuid,
    pub permission_code: String,
    pub permission_name :String,
    pub description: String,

    pub enabled:bool,
    pub create_time: chrono::DateTime<Local>,
    pub update_time: chrono::DateTime<Local>,
    pub data: Option<String>,
}

#[derive(Queryable)]
pub struct Role{
    pub id: i64,

    pub role_id: Uuid,
    pub role_code: String,
    pub role_name:String,

    pub permissions:String,
    pub description:String,
    pub enabled:bool,
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
    pub enabled:bool,
    pub create_time: chrono::DateTime<Local>,
    pub update_time: chrono::DateTime<Local>,
    pub data: Option<&'a str>,
}

#[derive(Queryable)]
pub struct Session{
    pub id: i64,
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub init_time: chrono::DateTime<Local>,
    pub expiry_time: chrono::DateTime<Local>,
    pub create_time: chrono::DateTime<Local>,
    pub update_time: chrono::DateTime<Local>,
    pub data: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name=sessions)]
pub struct NewSession<'a> {
    pub session_id: &'a Uuid,
    pub user_id: &'a Uuid,
    pub init_time: chrono::DateTime<Local>,
    pub expiry_time: chrono::DateTime<Local>,
    pub create_time: chrono::DateTime<Local>,
    pub update_time: chrono::DateTime<Local>,
    pub data: Option<&'a str>,
}

#[derive(Insertable)]
#[diesel(table_name=users)]
pub struct NewUser<'a>{
    pub user_id: &'a Uuid,
    pub description: &'a str,
    pub permissions: &'a str,
    pub roles: &'a str,
    pub enabled:bool,
    pub create_time: chrono::DateTime<Local>,
    pub update_time: chrono::DateTime<Local>,
    pub data: Option<&'a str>,
}

#[async_trait]
impl Authentication<User, AxumPgPool> for User{
    fn get_user(user_id:Uuid,pool:AxumPgPool)->User{
        let mut conn=pool.connection.lock().unwrap();//TODO error

        users::dsl::users
            .filter(users::dsl::user_id.eq(user_id))
            .filter(users::dsl::enabled.eq(true))
            .get_result::<User>(&mut *conn)
            .unwrap()
            //TODO error
    }

    fn load_identity(&self,pool:AxumPgPool) -> auth_user::Identity{
        let mut conn=pool.connection.lock().unwrap(); //TODO  unwrap error

        let user=users::dsl::users
            .filter(users::dsl::user_id.eq(self.user_id))
            .filter(users::dsl::enabled.eq(true))
            .get_result::<User>(&mut *conn)
            .unwrap();

        let permissions=permissions::dsl::permissions
            .filter(permissions::dsl::permission_id.eq_any(serde_json::from_str::<Vec<Uuid>>(&user.permissions).unwrap())) 
            .filter(permissions::dsl::enabled.eq(true))
            .get_results::<Permission>(&mut *conn)
            .unwrap();
        let roles=roles::dsl::roles
            .filter(roles::dsl::role_id.eq_any(serde_json::from_str::<Vec<Uuid>>(&user.roles).unwrap())) 
            .filter(roles::dsl::enabled.eq(true))
            .get_results::<Role>(&mut *conn)
            .unwrap();


        let identity=auth_user::Identity{
            user_id:user.user_id,
            roles:roles.into_iter().map(|r|auth_user::Role{
                role_id: r.role_id,
                role_code: r.role_code,
                role_name:r.role_name,

                permissions:r.permissions,
                description:r.description,
                enabled:r.enabled,
                create_time: r.create_time,
                update_time: r.update_time,
                data: r.data,
            }).collect(),
            permission_codes:permissions.iter().map(|p|p.permission_code.clone()).collect(),
            permissions:permissions.into_iter().map(|p|auth_user::Permission{
                permission_id: p.permission_id,
                    permission_code: p.permission_code,
                    permission_name :p.permission_name,
                    description: p.description,
                    enabled:p.enabled,
                    create_time: p.create_time,
                    update_time: p.update_time,
                    data: p.data,
            }).collect(),

        };

        identity
    }
}

#[derive(Queryable,Serialize)]
pub struct Consumer{
    #[serde(skip)]
    pub id: i64,

    pub user_id: Uuid,
    pub consumer_id: Uuid,
    pub cellphone:String,
    pub real_name:Option<String>,
    pub gender:Option<String>,
    pub birth_day:Option<NaiveDate>,

    #[serde(serialize_with = "custom_serialize")]
    pub balance:Cents,

    #[serde(skip)]
    pub enabled:bool,
    
    #[serde(with = "my_date_format")]
    pub create_time: chrono::DateTime<Local>,
    #[serde(with = "my_date_format")]
    pub update_time: chrono::DateTime<Local>,

    #[serde(skip)]
    pub data: Option<String>,
}

fn custom_serialize<S: serde::Serializer>(value: &Cents, ser: S) -> Result<S::Ok, S::Error> {
    ser.serialize_str(&format!("{:?}",value))
}

#[derive(Insertable)]
#[diesel(table_name=consumers)]
pub struct NewConsumer<'a>{
    pub user_id: &'a Uuid,
    pub consumer_id: &'a Uuid,
    pub cellphone:&'a str,
    pub real_name:Option<&'a str>,
    pub gender:Option<&'a str>,
    pub birth_day:Option<NaiveDate>,
    pub balance:Cents,
    pub enabled:bool,
    pub create_time: chrono::DateTime<Local>,
    pub update_time: chrono::DateTime<Local>,
    pub data: Option<&'a str>,
}

#[derive(Queryable,Serialize)]
pub struct Account{
    #[serde(skip)]
    pub id: i64,
    pub user_id: Uuid,
    pub account_id: Uuid,
    pub merchant_id: Uuid,

    pub cellphone:String,
    pub email:Option<String>,
    pub real_name:Option<String>,

    #[serde(skip)]
    pub enabled:bool,
    #[serde(with = "my_date_format")]
    pub create_time: chrono::DateTime<Local>,
    #[serde(with = "my_date_format")]
    pub update_time: chrono::DateTime<Local>,

    #[serde(skip)]
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
    pub real_name:Option<&'a str>,
    pub enabled:bool,
    pub create_time: chrono::DateTime<Local>,
    pub update_time: chrono::DateTime<Local>,
    pub data: Option<&'a str>,
}


#[derive(Queryable,Serialize)]
pub struct Merchant{
    #[serde(skip)]
    pub id: i64,

    pub merchant_id: Uuid,
    pub merchant_name:String,
    pub company_name:Option<String>,
    pub credential_no:Option<String>,

    #[serde(skip)]
    pub enabled:bool,
    #[serde(with = "my_date_format")]
    pub create_time: chrono::DateTime<Local>,
    #[serde(with = "my_date_format")]
    pub update_time: chrono::DateTime<Local>,

    #[serde(skip)]
    pub data: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name=merchants)]
pub struct NewMerchant<'a>{
    pub merchant_id: &'a Uuid,
    pub merchant_name:&'a str,
    pub company_name:Option<&'a str>,
    pub credential_no:Option<&'a str>,
    pub enabled:bool,
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
    pub update_time: chrono::DateTime<Local>,
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
    pub update_time: chrono::DateTime<Local>,
}

#[derive(Queryable)]
pub struct PasswordLoginProvider{
    pub id: i64,
    pub user_id: Uuid,
    pub password_hash: String,
    pub enabled: bool,
    pub create_time: chrono::DateTime<Local>,
    pub update_time: chrono::DateTime<Local>,
    pub data:Option<String>
}

#[derive(Insertable)]
#[diesel(table_name=password_login_providers)]
pub struct NewPasswordLoginProvider<'a>{
    pub user_id: &'a Uuid,
    pub password_hash: &'a str,
    pub enabled: bool,
    pub create_time: chrono::DateTime<Local>,
    pub update_time: chrono::DateTime<Local>,
    pub data:Option<&'a str>
}
