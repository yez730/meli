use std::collections::HashMap;

use chrono::{DateTime, Local, Duration};
use uuid::Uuid;

#[derive(Debug,Clone)]
pub(crate) struct AxumSessionData{
    pub(crate) session_id:Uuid,

    // 登录用户 / 匿名用户
    pub(crate) user_id:Option<Uuid>,
    pub(crate) init_time:DateTime<Local>,
    pub(crate) expiry_time:DateTime<Local>,
    pub(crate) data:HashMap<String,String>,
}

impl AxumSessionData{
    pub(crate) fn init(session_id:Uuid,memory_clear_timeout:Duration)->AxumSessionData{
        AxumSessionData{
            session_id,
            user_id:None,
            init_time:Local::now(),
            expiry_time:Local::now()+memory_clear_timeout, //匿名用户临时保存在内存中的有效时间
            data:HashMap::new(),
        }
    }
}
