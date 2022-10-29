use std::env;

use axum::{extract::State,http::StatusCode, Json};
use axum_session_authentication_middleware::session::AuthSession;
use chrono::Local;
use dotenvy::dotenv;
use email_address::EmailAddress;
use regex::Regex;
use serde::Deserialize;
use uuid::Uuid;

use crate::{axum_pg::AxumPg, models::{User, NewMerchant, NewUser, NewLoginInfo, NewPasswordLoginProvider, NewBarber, Barber, Merchant, LoginInfo}, schema::{login_infos, merchants, users, password_login_providers, barbers}, authorization_policy, constant, regex_constants::CELLPHONE_REGEX_STRING};
use diesel::{
    prelude::*,
    select, 
    dsl::exists,
};
use crate::schema::*;
use super::barber::BarberResponse; 

#[derive(Deserialize)]
pub struct RegisterMerchantRequest{
    #[serde(rename="merchantName")]
    pub merchant_name:String,

    #[serde(rename="loginAccount")]
    pub login_account:String,

    #[serde(rename="accountRealName")]
    pub account_real_name:String,

    pub password:String,
}

pub async fn register_merchant(State(pg):State<AxumPg>,mut auth: AuthSession<AxumPg, AxumPg,User>,Json(req):Json<RegisterMerchantRequest>)->Result<Json<BarberResponse>,(StatusCode,String)>{
    let mut conn=pg.pool.get().unwrap();
    
    let login_info_type;
    if EmailAddress::is_valid(req.login_account.as_str()){
        login_info_type="Email";
    } else if Regex::new(CELLPHONE_REGEX_STRING).unwrap().is_match(req.login_account.as_str()){
        login_info_type="Cellphone";
    } else {
        return Err((StatusCode::BAD_REQUEST,"手机号或邮箱格式不正确".to_string()));
    }

    let existed_merchant=select(exists(
            merchants::table
            .filter(merchants::enabled.eq(true))
            .filter(merchants::merchant_name.eq(&req.merchant_name))
        ))
        .get_result(&mut *conn)
        .unwrap();
    if existed_merchant{
        return Err((StatusCode::BAD_REQUEST,"该商户名已存在".to_string()));
    }

    let new_merchant=NewMerchant{
        merchant_id: &Uuid::new_v4(),
        merchant_name:req.merchant_name.as_ref(),
        company_name:None,
        credential_no:None,
        enabled:true,
        create_time: Local::now(),
        update_time: Local::now(),
        data: None,
        address:None,
        remark:None,
    };
    let merchant=diesel::insert_into(merchants::table)
        .values(&new_merchant)
        .get_result::<Merchant>(&mut *conn)
        .unwrap();

    let user:User;

    //TODO how 验证邮箱密码有效性？

    let login_info_user=login_infos::table.inner_join(users::table.on(login_infos::user_id.eq(users::user_id)))
        .filter(login_infos::enabled.eq(true))
        .filter(users::enabled.eq(true))
        .filter(login_infos::login_info_account.eq(&req.login_account))
        .get_result::<(LoginInfo,User)>(&mut *conn)
        .ok();

    if let Some((_,existed_user))=login_info_user{
        user=existed_user;
    } else {
        let user_description=format!("Administrator of merchant {}",req.merchant_name);
        let new_user=NewUser{
            user_id: &Uuid::new_v4(),
            description: user_description.as_str(),
            permissions:"[]",
            roles:"[]",
            enabled:true,
            create_time: Local::now(),
            update_time: Local::now(),
            data: None,
        };
        user=diesel::insert_into(users::table)
            .values(&new_user)
            .get_result::<User>(&mut *conn)
            .unwrap();

        let login_info=NewLoginInfo{
            login_info_id: &Uuid::new_v4(),
            login_info_account: &req.login_account,
            login_info_type, 
            user_id: &user.user_id,
            enabled: true, 
            create_time: Local::now(),
            update_time: Local::now(),
        };
        diesel::insert_into(login_infos::table)
            .values(&login_info)
            .execute(&mut *conn)
            .unwrap();
    }
   
    let new_barber=NewBarber{
        user_id:  &user.user_id,
        barber_id: &Uuid::new_v4(),
        merchant_id:&new_merchant.merchant_id,
        email:if login_info_type=="Email" {Some(req.login_account.as_ref())} else {None},
        cellphone:if login_info_type=="Cellphone" {Some(req.login_account.as_ref())} else {None},
        real_name:req.account_real_name.as_ref(),
        enabled:true,
        create_time: Local::now(),
        update_time: Local::now(),
        data: None,
    };
    let barber=diesel::insert_into(barbers::table)
        .values(&new_barber)
        .get_result::<Barber>(&mut *conn)
        .unwrap();

    // add permissions
    let mut ids=serde_json::from_str::<Vec<String>>(&user.permissions).unwrap();
    for &permission_code in authorization_policy::ADMINISTRATOR_PERMISSIONS_OF_MERCHANT_BARBER{
        let permission_id=permissions::table
            .filter(permissions::permission_code.eq(permission_code)) 
            .filter(permissions::enabled.eq(true))
            .select(permissions::permission_id)
            .get_result::<Uuid>(&mut *conn)
            .unwrap();

        ids.push(format!("{}:{}",new_merchant.merchant_id,permission_id));
    }
    let administrator_permission_id=permissions::table
        .filter(permissions::permission_code.eq(authorization_policy::MERCHANT_ADMINISTRATOR)) 
        .filter(permissions::enabled.eq(true))
        .select(permissions::permission_id)
        .get_result::<Uuid>(&mut *conn)
        .unwrap();
    ids.push(format!("{}:{}",new_merchant.merchant_id,administrator_permission_id));

    let barber_base_permission_id=permissions::table
        .filter(permissions::permission_code.eq(authorization_policy::BARBER_BASE)) 
        .filter(permissions::enabled.eq(true))
        .select(permissions::permission_id)
        .get_result::<Uuid>(&mut *conn)
        .unwrap();
    ids.push(format!("{}:{}",new_merchant.merchant_id,barber_base_permission_id));

    diesel::update(
        users::table
        .filter(users::user_id.eq(user.user_id))
        .filter(users::enabled.eq(true))
    )
    .set((
        users::permissions.eq(serde_json::to_string(&ids).unwrap()),
        users::update_time.eq(Local::now())
    ))
    .execute(&mut *conn)
    .unwrap();


    //TODO how 已注册用户的禁止再修改密码？
    dotenv().expect("Cannot find .env file.");
    let salt=env::var("DATABASE_ENCRYPTION_SAULT").unwrap();
    let config = argon2::Config::default();
    let hash = argon2::hash_encoded(req.password.as_bytes(), salt.as_bytes(), &config).unwrap();
    let new_password_login_provider=NewPasswordLoginProvider{
        user_id: &user.user_id,
        password_hash: &hash,
        enabled:true,
        create_time: Local::now(),
        update_time: Local::now(),
        data:None
    };
    diesel::insert_into(password_login_providers::table)
        .values(&new_password_login_provider)
        .execute(&mut *conn).map_err(|e|{
            tracing::error!("{}",e.to_string());
            (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
        })?;
        
    let merchant_id=new_merchant.merchant_id;
    auth.sign_in(user.user_id,Some(merchant_id.to_owned())).await;
    auth.axum_session.lock().unwrap().set_data(constant::MERCHANT_ID.to_owned(), merchant_id.to_string());

    Ok(Json(BarberResponse{barber,merchant}))
}

// pub async fn register_barber(){
    
// }

// pub async fn register_member(){
    
// }
