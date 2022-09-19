pub mod models;
pub mod schema;
pub mod axum_pg_pool;
pub mod utils;
pub mod login_managers;
pub mod authorization_policy;
pub mod handlers;

use chrono::Local;
use models::*;
use uuid::Uuid;

use diesel::PgConnection;
use diesel::prelude::*;

use crate::login_managers::LoginInfoType;

pub fn create_or_update_super_user_account(conn:&mut PgConnection){
    use crate::schema::*;

    // 1. insert merchant
    let merchant_id=Uuid::new_v4();
    let new_merchant=NewMerchant{
        merchant_id: &merchant_id,
        merchant_name:"测试商户",
        company_name:None,
        credential_no:None,
        enabled:true,
        create_time: Local::now(),
        update_time: Local::now(),
        data: None,
    };
    diesel::insert_into(merchants::table)
    .values(&new_merchant)
    .execute(conn)
    .unwrap();
    
    // 2.1 insert user
    let mut perms=authorization_policy::DEFAULT_PERMISSIONS_OF_MERCHANT_ACCOUNT.to_vec();
    perms.push(authorization_policy::ACCOUNT); //商户用户权限
    let user_id=Uuid::new_v4();
    let new_user=NewUser{
        user_id: &user_id,
        description: "test user",
        permissions:&serde_json::to_string(&perms).unwrap(),
        roles:"[]",
        enabled:true,
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
        real_name:Some("方小会"),
        enabled:true,
        create_time: Local::now(),
        update_time: Local::now(),
        data: None,
    };
    diesel::insert_into(accounts::table)
    .values(&new_account)
    .execute(conn)
    .unwrap();

    // 3.1 add login info
    let l_i_1= NewLoginInfo{
        login_info_id: &Uuid::new_v4(),
        login_info_account: "13764197590",
        login_info_type: "Cellphone",
        user_id: &user_id,
        enabled: true,
        create_time: Local::now(),
        update_time: Local::now(),
    };
    let l_i_2=NewLoginInfo{
        login_info_id: &Uuid::new_v4(),
        login_info_account: "yez",
        login_info_type: "Username",
        user_id: &user_id,
        enabled: true,
        create_time: Local::now(),
        update_time: Local::now(),
    };
    diesel::insert_into(login_infos::table)
    .values(&vec![l_i_1,l_i_2])
    .execute(conn)
    .unwrap();

    // 3.2 add password login info provider
    let password = b"123456";
    let salt = b"randomsalt";
    let config = argon2::Config::default();
    let hash = argon2::hash_encoded(password, salt, &config).unwrap();
    let new_password_login_provider=NewPasswordLoginProvider{
        user_id: &user_id,
        password_hash: &hash,
        enabled:true,
        create_time: Local::now(),
        update_time: Local::now(),
        data:None
    };
    diesel::insert_into(password_login_providers::table)
    .values(&new_password_login_provider)
    .execute(conn)
    .unwrap();
}

#[allow(dead_code)]
mod my_date_format {
    use chrono::{DateTime, Local, TimeZone};
    use serde::{self, Deserialize, Serializer, Deserializer};

    // const FORMAT: &str = "%+";
    const FORMAT: &'static str = "%Y-%m-%d %H:%M:%S";

    pub fn serialize<S>(
        date: &Option<DateTime<Local>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match date {
            Some(value) => serializer.serialize_some(&format!("{}",value.format(FORMAT))),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<DateTime<Local>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let option:Option<String>=Option::deserialize(deserializer)?;
        option.map(|s|Local
            .datetime_from_str(&s, FORMAT)
            .map_err(serde::de::Error::custom)
        )
        .transpose()
    }
}

#[cfg(test)]
mod test{
    use serde::{Serialize,Deserialize};
    use chrono::DateTime;
    use super::{*,my_date_format};

    #[test]
    #[ignore]
    fn test_create_or_update_super_user_account(){
        create_or_update_super_user_account(&mut utils::get_connection());
    }

    #[derive(Deserialize,Serialize)]
    struct Input{
        a:String,
        b:Option<String>,

        #[serde(default, with = "my_date_format")]
        c:Option<DateTime<Local>>,
    }
    #[test]
    fn test_serde(){
        // use chrono::{DateTime, TimeZone, NaiveDateTime, Local};
        // let local1=Local.ymd(2015, 5, 15);
        // println!("{}",local1); //2015-05-15+08:00

        // let input1=Input{a:"123".into(),b:None,c:None};
        // let json1=serde_json::to_string(&input1).unwrap();
        // assert_eq!(json1,"");

        let input2=Input{a:"123".into(),b:Some("456".into()),c:Some(Local::now())};
        let json2=serde_json::to_string(&input2).unwrap();
        assert_eq!(json2,"");
        // my_date_format::serialize(date, serializer)

        // let json3=r#"{"a":"123","b":"456","c":"2022-09-19 22:51:32"}"#;
        // let input3=serde_json::from_str::<Input>(json3).unwrap();
        // assert_eq!(input3.a,"123");
        // assert_eq!(input3.b,Some("456".into()));
        // assert_eq!(input3.c,Some(Local::now()));

        // let json4=r#"{"a":"123","b":null}"#;
        // let json4=r#"{"a":"123"}"#;
        // let input4=serde_json::from_str::<Input>(json4).unwrap();
        // assert_eq!(input4.a,"123");
        // assert_eq!(input4.b,None);
    }
}
