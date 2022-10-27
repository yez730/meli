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
    fn load_identity(user_id:Uuid,merchant_id:Option<Uuid>,pg:AxumPg) -> auth_user::Identity{
        let mut conn=pg.pool.get().unwrap();

        let user=users::table
            .filter(users::user_id.eq(user_id))
            .filter(users::enabled.eq(true))
            .get_result::<User>(&mut *conn)
            .unwrap();

        let mut current_permission_ids=Vec::new();
        let ids=serde_json::from_str::<Vec<String>>(&user.permissions).unwrap();
        for id_str in ids {
            let mut id_iter=id_str.split(':'); // `merchant_id:permission_id` or `permission_id`

            let id1=Uuid::parse_str(id_iter.next().unwrap()).unwrap();
            let id2=id_iter.next();
            
            match id2 {
                Some(permission_id) if Some(id1)==merchant_id=>{
                    current_permission_ids.push(Uuid::parse_str(permission_id).unwrap());
                }
                None=>{
                    current_permission_ids.push(id1);
                }
                _=>{}
            }
        }

        let permissions=permissions::table
            .filter(permissions::permission_id.eq_any(current_permission_ids))
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
pub struct Permission{
    #[serde(skip)]
    pub id: i64,

    #[serde(rename="permissionId")]
    pub permission_id: Uuid,

    #[serde(rename="permissionCode")]
    pub permission_code: String,

    #[serde(rename="permissionName")]
    pub permission_name :String,

    pub description: String,

    #[serde(skip)]
    pub enabled:bool,

    #[serde(rename="createTime")]
    pub create_time: chrono::DateTime<Local>,

    #[serde(rename="updateTime")]
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
pub struct Member{
    #[serde(skip)]
    pub id: i64,

    #[serde(rename ="userId")]
    pub user_id: Uuid,

    #[serde(rename ="memberId")]
    pub member_id: Uuid,
    
    pub cellphone:String,
    
    #[serde(rename ="realName")]
    pub real_name:String,

    pub gender:Option<String>,

    #[serde(rename ="birthDay")]
    pub birth_day:Option<NaiveDate>,

    #[serde(skip)]
    pub enabled:bool,

    #[serde(with = "my_date_format",rename ="createTime")]
    pub create_time: chrono::DateTime<Local>,

    #[serde(with = "my_date_format",rename ="updateTime")]
    pub update_time: chrono::DateTime<Local>,

    #[serde(skip)]
    pub data: Option<String>,
        
    pub remark:Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name=members)]
pub struct NewMember<'a>{
    pub user_id: &'a Uuid,
    pub member_id: &'a Uuid,
    pub cellphone:&'a str,
    pub real_name:&'a str,
    pub gender:Option<&'a str>,
    pub birth_day:Option<NaiveDate>,
    pub enabled:bool,
    pub create_time: chrono::DateTime<Local>,
    pub update_time: chrono::DateTime<Local>,
    pub data: Option<&'a str>,
    
    pub remark:Option<&'a str>,
}

#[derive(Queryable,Serialize)]
pub struct MerchantMember{
    #[serde(skip)]
    pub id: i64,

    #[serde(rename ="merchantId")]
    pub merchant_id: Uuid,

    #[serde(skip)]
    pub member_id: Uuid,

    pub balance:BigDecimal,
    
    #[serde(skip)]
    pub enabled:bool,
    
    #[serde(with = "my_date_format",rename ="createTime")]
    pub create_time: chrono::DateTime<Local>,

    #[serde(with = "my_date_format",rename ="updateTime")]
    pub update_time: chrono::DateTime<Local>,

    #[serde(skip)]
    pub data: Option<String>,
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
}

#[derive(Queryable,Serialize,Clone)]
pub struct Barber{
    #[serde(skip)]
    pub id: i64,

    #[serde(rename ="userId")]
    pub user_id: Uuid,
    
    #[serde(rename ="barberId")]
    pub barber_id: Uuid,
    
    #[serde(rename ="merchantId")]
    pub merchant_id: Uuid,

    pub cellphone:Option<String>,

    pub email:Option<String>,

    #[serde(rename ="realName")]
    pub real_name:String,

    #[serde(skip)]
    pub enabled:bool,

    #[serde(with = "my_date_format",rename ="createTime")]
    pub create_time: chrono::DateTime<Local>,
    
    #[serde(with = "my_date_format",rename ="updateTime")]
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
pub struct Merchant{
    #[serde(skip)]
    pub id: i64,

    #[serde(rename ="merchantId")]
    pub merchant_id: Uuid,

    #[serde(rename ="merchantName")]
    pub merchant_name:String,

    #[serde(rename ="companyName")]
    pub company_name:Option<String>,

    #[serde(rename ="credentialNo")]
    pub credential_no:Option<String>,

    #[serde(skip)]
    pub enabled:bool,

    #[serde(with = "my_date_format",rename ="createTime")]
    pub create_time: chrono::DateTime<Local>,
    
    #[serde(with = "my_date_format",rename ="updateTime")]
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
pub struct ServiceType{
    #[serde(skip)]
    pub id: i64,

    #[serde(rename ="serviceTypeId")]
    pub service_type_id: Uuid,

    #[serde(rename ="merchantId")]
    pub merchant_id: Uuid,

    pub name: String,

    #[serde(rename ="estimatedDuration")]
    pub estimated_duration: i32,

    #[serde(rename ="normalPrize")]
    pub normal_prize:BigDecimal,

    #[serde(rename ="memberPrize")]
    pub member_prize:BigDecimal,

    #[serde(skip)]
    pub enabled:bool,

    #[serde(with = "my_date_format",rename ="createTime")]
    pub create_time: chrono::DateTime<Local>,

    #[serde(with = "my_date_format",rename ="updateTime")]
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
pub struct RechargeRecord{
    #[serde(skip)]
    pub id: i64,

    #[serde(rename ="rechargeRecordId")]
    pub recharge_record_id: Uuid,

    #[serde(rename ="merchantId")]
    pub merchant_id: Uuid,

    #[serde(rename ="memberId")]
    pub member_id: Uuid,

    pub amount:BigDecimal,

    #[serde(rename ="barberId")]
    pub barber_id:Uuid,

    #[serde(skip)]
    pub enabled:bool,

    #[serde(with = "my_date_format",rename ="createTime")]
    pub create_time: chrono::DateTime<Local>,
    
    #[serde(with = "my_date_format",rename ="updateTime")]
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
pub struct Order{
    #[serde(skip)]
    pub id: i64,

    #[serde(rename = "id")]
    pub order_id: Uuid,

    #[serde(rename = "merchantId")]
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
