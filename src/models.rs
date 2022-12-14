use async_trait::async_trait;
use axum_session_authentication_middleware::{ user as auth_user,session::Authentication};
use chrono::{Local, NaiveDate};
use diesel::prelude::*;
use serde::Serialize;
use uuid::Uuid;
use bigdecimal::BigDecimal;

use crate::{schema::*, axum_pg::AxumPg, my_date_format};

#[derive(Queryable,Clone, Debug)]
pub struct User{
    pub id: i64,

    pub user_id: Uuid,
    pub description: String,
    pub permissions: String, // 属于 merchant 则为 `merchant_id:permission_id`，否则 `permission_id`
    pub roles: String,
    pub enabled:bool,

    pub create_time: chrono::DateTime<Local>,
    pub update_time: chrono::DateTime<Local>,

    pub data: Option<String>,
}

#[async_trait]
impl Authentication<User, AxumPg> for User{
    fn load_identity(user_id:Uuid,pg:AxumPg) -> auth_user::Identity{
        let mut conn=pg.pool.get().unwrap();

        let user=users::table
            .filter(users::user_id.eq(user_id))
            .filter(users::enabled.eq(true))
            .get_result::<User>(&mut *conn)
            .unwrap();

        let permissions=permissions::table
            .filter(permissions::permission_id.eq_any(serde_json::from_str::<Vec<Uuid>>(&user.permissions).unwrap()))
            .filter(permissions::enabled.eq(true))
            .get_results::<Permission>(&mut *conn)
            .unwrap();
        let roles=roles::table
            .filter(roles::role_id.eq_any(serde_json::from_str::<Vec<Uuid>>(&user.roles).unwrap())) 
            .filter(roles::enabled.eq(true))
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

#[derive(Queryable,Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Permission{
    #[serde(skip)]
    pub id: i64,

    pub permission_id: Uuid,

    pub permission_code: String,

    pub permission_name :String,

    pub description: String,

    #[serde(skip)]
    pub enabled:bool,

    pub create_time: chrono::DateTime<Local>,

    pub update_time: chrono::DateTime<Local>,

    #[serde(skip)]
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
    pub data: String,
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
    pub data: &'a str,
}

#[derive(Queryable,Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MerchantMember{
    #[serde(skip)]
    pub id: i64,

    pub merchant_id: Uuid,

    pub member_id: Uuid,

    pub balance:BigDecimal,
    
    #[serde(skip)]
    pub enabled:bool,
    
    #[serde(with = "my_date_format")]
    pub create_time: chrono::DateTime<Local>,

    #[serde(with = "my_date_format")]
    pub update_time: chrono::DateTime<Local>,

    #[serde(skip)]
    pub data: Option<String>,

    pub cellphone:String,
    pub real_name:String,
    pub gender:Option<String>,
    pub birth_day:Option<NaiveDate>,
    pub remark:Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name=merchant_members)]
pub struct NewMerchantMember<'a>{
    pub merchant_id: &'a Uuid,
    pub member_id: &'a Uuid,
    pub balance:&'a BigDecimal,
    pub enabled:bool,
    pub create_time: chrono::DateTime<Local>,
    pub update_time: chrono::DateTime<Local>,
    pub data: Option<&'a str>,

    pub cellphone:&'a str,
    pub real_name:&'a str,
    pub gender:Option<&'a str>,
    pub birth_day:Option<NaiveDate>,
    pub remark:Option<&'a str>,
}

#[derive(Queryable,Serialize,Clone)]
#[serde(rename_all = "camelCase")]
pub struct Barber{
    #[serde(skip)]
    pub id: i64,

    pub user_id: Uuid,
    
    pub barber_id: Uuid,
    
    pub merchant_id: Uuid,

    pub cellphone:Option<String>,

    pub email:Option<String>,

    pub real_name:String,

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
#[diesel(table_name=barbers)]
pub struct NewBarber<'a>{
    pub user_id: &'a Uuid,
    pub barber_id: &'a Uuid,
    pub merchant_id: &'a Uuid,
    pub cellphone:Option<&'a str>,
    pub email:Option<&'a str>,
    pub real_name:&'a str,
    pub enabled:bool,
    pub create_time: chrono::DateTime<Local>,
    pub update_time: chrono::DateTime<Local>,
    pub data: Option<&'a str>,
}

#[derive(Queryable,Serialize,Clone)]
#[serde(rename_all = "camelCase")]
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

    pub address:Option<String>,

    pub remark:Option<String>,
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

    pub address:Option<&'a str>,
    pub remark:Option<&'a str>,
}

#[derive(Queryable,Clone)]
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

#[derive(Queryable,Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceType{
    #[serde(skip)]
    pub id: i64,

    pub service_type_id: Uuid,

    pub merchant_id: Uuid,

    pub name: String,

    pub estimated_duration: i32,

    pub normal_prize:BigDecimal,

    pub member_prize:BigDecimal,

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
#[diesel(table_name=service_types)]
pub struct NewServiceType<'a>{
    pub service_type_id: &'a Uuid,
    pub merchant_id: &'a Uuid,
    pub name:&'a str,
    pub estimated_duration: i32,
    pub normal_prize:&'a BigDecimal,
    pub member_prize:&'a BigDecimal,
    pub enabled:bool,
    pub create_time: chrono::DateTime<Local>,
    pub update_time: chrono::DateTime<Local>,
    pub data: Option<&'a str>,
}


#[derive(Queryable,Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RechargeRecord{
    #[serde(skip)]
    pub id: i64,

    pub recharge_record_id: Uuid,

    pub merchant_id: Uuid,

    pub member_id: Uuid,

    pub amount:BigDecimal,

    pub barber_id:Uuid,

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
#[diesel(table_name=recharge_records)]
pub struct NewRechargeRecord<'a>{
    pub recharge_record_id: &'a Uuid,
    pub merchant_id: &'a Uuid,
    pub member_id: &'a Uuid,
    pub amount:&'a BigDecimal,

    pub barber_id: &'a Uuid,
    pub enabled:bool,
    pub create_time: chrono::DateTime<Local>,
    pub update_time: chrono::DateTime<Local>,
    pub data: Option<&'a str>,
}

#[derive(Queryable,Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Order{
    #[serde(skip)]
    pub id: i64,

    #[serde(rename = "id")]
    pub order_id: Uuid,

    pub merchant_id: Uuid,

    #[serde(rename="start", with = "my_date_format")]
    pub start_time: chrono::DateTime<Local>,

    #[serde(rename="end",with = "my_date_format")]
    pub end_time: chrono::DateTime<Local>,
    
    #[serde(skip)]
    pub consumer_type:String,  // walk-in / member
    
    #[serde(skip)]
    pub member_id: Option<Uuid>,
    
    #[serde(skip)]
    pub barber_id:Uuid,
    
    #[serde(skip)]
    pub service_type_id:Uuid,
    
    #[serde(skip)]
    pub status:String,
    
    #[serde(skip)]
    pub payment_type:String, // member / cash
    
    #[serde(skip)]
    pub amount:BigDecimal,
    
    #[serde(skip)]
    pub remark:Option<String>,

    #[serde(skip)]
    pub enabled:bool,
    
    #[serde(skip)]
    pub create_time: chrono::DateTime<Local>,
    
    #[serde(skip)]
    pub update_time: chrono::DateTime<Local>,

    #[serde(skip)]
    pub data: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name=orders)]
pub struct NewOrder<'a>{
    pub order_id: &'a Uuid,
    pub merchant_id: &'a Uuid,
    pub start_time:chrono::DateTime<Local>,
    pub end_time:chrono::DateTime<Local>,
    pub consumer_type:&'a str,  // walk-in / member
    pub member_id: Option<&'a Uuid>,
    pub barber_id: &'a Uuid,
    pub service_type_id:&'a Uuid,
    pub status:&'a str,
    pub payment_type:&'a str, // member / cash
    pub amount:&'a BigDecimal,
    pub remark:Option<&'a str>,

    pub enabled:bool,
    pub create_time: chrono::DateTime<Local>,
    pub update_time: chrono::DateTime<Local>,
    pub data: Option<&'a str>,
}
