use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, Local};
use uuid::Uuid;

#[async_trait]
pub trait AxumDatabaseTrait{
    async fn store(&self,session_data:&SessionData) -> Result<(), anyhow::Error>;

    async fn load(&self, session_id: &Uuid) -> Result<SessionData, anyhow::Error>;
}

pub struct SessionData{
    pub session_id:Uuid,
    pub user_id:Uuid,
    pub init_time:DateTime<Local>,
    pub expiry_time:DateTime<Local>,
    pub data:HashMap<String,String>,
}