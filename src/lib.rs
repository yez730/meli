pub mod models;
pub mod schema;
pub mod axum_pg_pool;
pub mod util;
pub mod login_managers;
pub mod authorization_policy;
pub mod handlers;

use chrono::Local;
use models::*;
use uuid::Uuid;

use diesel::PgConnection;
use diesel::prelude::*;

use crate::authorization_policy::DEFAULT_PERMISSIONS_OF_MERCHANT_ACCOUNT;


pub fn create_or_update_super_user_account(conn:&mut PgConnection){
    use crate::schema::{*};

    // 1. insert merchant
    let merchant_id=Uuid::new_v4();
    let new_merchant=NewMerchant{
        merchant_id: &merchant_id,
        merchant_name:"测试商户",
        company_name:None,
        credential_no:None,
        create_time: Local::now(),
        update_time: Local::now(),
        data: None,
    };
    diesel::insert_into(merchants::table)
    .values(&new_merchant)
    .execute(conn)
    .unwrap();
    
    // 2.1 insert user
    let user_id=Uuid::new_v4();
    let new_user=NewUser{
        user_id: &user_id,
        username: "yez",
        description: "",
        permissions:&serde_json::to_string(DEFAULT_PERMISSIONS_OF_MERCHANT_ACCOUNT).unwrap(),
        roles:"[]",
        create_time: Local::now(),
        update_time: Local::now(),
        data: None,
    };
    diesel::insert_into(users::table)
    .values(&new_user)
    .execute(conn)
    .unwrap();

    // 2.1 insert account
    let new_account=NewAccount{
        user_id: &user_id,
        account_id: &Uuid::new_v4(),
        merchant_id: &merchant_id,
        cellphone:"13764197590",
        email:None,
        credential_no:None,
        real_name:Some("方小会"),
        create_time: Local::now(),
        update_time: Local::now(),
        data: None,
    };
    diesel::insert_into(accounts::table)
    .values(&new_account)
    .execute(conn)
    .unwrap();

    // 3.1 add login info
    let new_login_info=NewLoginInfo{
        login_info_id: &Uuid::new_v4(),
        login_info_account: "13764197590",
        login_info_type: "Cellphone",
        user_id: &user_id,
        enabled: true,
        create_time: Local::now(),
    };
    diesel::insert_into(login_infos::table)
    .values(&new_login_info)
    .execute(conn)
    .unwrap();

    // 3.2 add password login info provider
    let new_password_login_provider=NewPasswordLoginProvider{
        user_id: &user_id,
        password_hash: "123456", //TODO do hash
        create_time: Local::now(),
        update_time: Local::now(),
        data:None
    };
    diesel::insert_into(password_login_providers::table)
    .values(&new_password_login_provider)
    .execute(conn)
    .unwrap();
}

#[cfg(test)]
mod test{
use super::*;

    #[test]
    fn test_create_or_update_super_user_account(){
        create_or_update_super_user_account(&mut util::get_connection());
        assert!(true);
    }
}
