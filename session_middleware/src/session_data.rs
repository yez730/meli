use std::collections::HashMap;

use chrono::{DateTime, Local, Duration};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug,Clone)]
pub struct AxumSessionData{
    pub session_id:Uuid,

    //Some: 登录用户 / None: 匿名用户
    pub user_id:Option<Uuid>,
    pub init_time:DateTime<Local>,
    pub expiry_time:DateTime<Local>,
    pub data:HashMap<String,String>,
}

impl AxumSessionData{
    pub fn init(session_id:Uuid,memory_clear_timeout:Duration)->AxumSessionData{
        AxumSessionData{
            session_id,
            user_id:None,
            init_time:Local::now(),
            expiry_time:Local::now()+memory_clear_timeout, //匿名用户临时保存在内存中的有效时间
            data:HashMap::new(),
        }
    }
}
