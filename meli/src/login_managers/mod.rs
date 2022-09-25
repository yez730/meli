use crate::{schema::*, models::LoginInfo};
use diesel::prelude::*;
use anyhow::anyhow;

//TODO how to `variant.as_str()`
pub enum LoginInfoType{
    Cellphone,
    Email,
    Username,
}

//通过登录名、手机号、邮箱或其它唯一标识和登录方式获取登录信息
pub fn get_login_info(account:String,conn:&mut PgConnection)->Result<LoginInfo,anyhow::Error>{
    login_infos::dsl::login_infos
        .filter(login_infos::login_info_account.eq(account))
        .filter(login_infos::enabled.eq(true))
        .get_result::<LoginInfo>(conn)
        .map_err(|e|anyhow!(e.to_string()))
}

pub mod password_login{
    use anyhow::{Ok, anyhow};
    use uuid::Uuid;
    use diesel::prelude::*;
    use crate::{schema::*, models::PasswordLoginProvider};

    pub fn verify_password(u_id:Uuid,password:String,conn:&mut PgConnection) -> Result<(),anyhow::Error>{
        let provider=password_login_providers::dsl::password_login_providers
            .filter(password_login_providers::dsl::user_id.eq(u_id))
            .filter(password_login_providers::dsl::enabled.eq(true))
            .get_result::<PasswordLoginProvider>(conn)?;
           
        let verified=argon2::verify_encoded(&provider.password_hash, password.as_bytes()).map_err(|e|anyhow!(e.to_string()))?;
        if !verified{
            return Err(anyhow!("密码验证失败"));
        }

        Ok(())
    }
}
