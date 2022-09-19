use crate::{schema::*, models::LoginInfo, utils::get_connection};
use diesel::prelude::*;

//TODO how to `variant.as_str()`
pub enum LoginInfoType{
    Cellphone,
    Email,
    Username,
}

//通过登录名、手机号、邮箱或其它唯一标识和登录方式获取登录信息
pub fn get_login_info(account:String)->Option<LoginInfo>{
    login_infos::dsl::login_infos
        .filter(login_infos::login_info_account.eq(account))
        .filter(login_infos::enabled.eq(true))
        .get_result::<LoginInfo>(&mut get_connection())
        .ok()
}

pub mod password_login{
    use uuid::Uuid;
    use diesel::prelude::*;
    use crate::{schema::*, models::PasswordLoginProvider, utils::get_connection};

    pub fn verify_password(u_id:Uuid,password:String) -> bool{
        let provider=password_login_providers::dsl::password_login_providers
            .filter(password_login_providers::dsl::user_id.eq(u_id))
            .filter(password_login_providers::dsl::enabled.eq(true))
            .get_result::<PasswordLoginProvider>(&mut get_connection()).ok();
           
        match provider {
            Some(provder)=>{
                argon2::verify_encoded(&provder.password_hash, password.as_bytes()).unwrap()
            }
            None=>false
        }
    }
}
