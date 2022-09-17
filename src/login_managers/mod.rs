use crate::{schema::login_infos::{dsl::*}, models::LoginInfo, util::get_connection};
use diesel::prelude::*;

pub enum LoginInfoType{
    Cellphone,
    Email,
    UserName,
}

//通过登录名、手机号、邮箱或其它唯一标识和登录方式获取登录信息
pub fn get_login_info(account:String)->Option<LoginInfo>{
    login_infos
        .filter(login_info_account.eq(account))
        .filter(enabled.eq(true))
        .get_result::<LoginInfo>(&mut get_connection())
        .ok()
}

pub mod password_login{
    use uuid::Uuid;
    use diesel::prelude::*;
    use crate::{schema::password_login_providers::dsl::*, models::PasswordLoginProvider, util::get_connection};

    pub fn verify_password(u_id:Uuid,password:String) -> bool{
        let provder=password_login_providers
            .filter(user_id.eq(u_id))
            .get_result::<PasswordLoginProvider>(&mut get_connection()).ok();

        match provder {
            Some(provder)=>provder.password_hash==password, //TODO password password
            None=>false
        }
    }
}
