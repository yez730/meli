use crate::{schema::*, models::LoginInfo, utils::get_connection};
use diesel::prelude::*;
use anyhow::anyhow;

//TODO how to `variant.as_str()`
pub enum LoginInfoType{
    Cellphone,
    Email,
    Username,
}

//通过登录名、手机号、邮箱或其它唯一标识和登录方式获取登录信息
pub fn get_login_info(account:String)->Result<LoginInfo,anyhow::Error>{
    login_infos::dsl::login_infos
        .filter(login_infos::login_info_account.eq(account))
        .filter(login_infos::enabled.eq(true))
        .get_result::<LoginInfo>(&mut get_connection())
        .map_err(|e|anyhow!(e.to_string()))
}

pub mod password_login{
    use anyhow::{Ok, anyhow};
    use uuid::Uuid;
    use diesel::prelude::*;
    use crate::{schema::*, models::PasswordLoginProvider, utils::get_connection};

    pub fn verify_password(u_id:Uuid,password:String) -> Result<(),anyhow::Error>{
        let provider=password_login_providers::dsl::password_login_providers
            .filter(password_login_providers::dsl::user_id.eq(u_id))
            .filter(password_login_providers::dsl::enabled.eq(true))
            .get_result::<PasswordLoginProvider>(&mut get_connection())?;
           
        argon2::verify_encoded(&provider.password_hash, password.as_bytes()).map_err(|e|anyhow!(e.to_string()))?;
        
        Ok(())
    }
}
