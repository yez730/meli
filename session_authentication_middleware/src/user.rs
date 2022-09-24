use chrono::Local;
use serde::Serialize;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Serialize,Debug,Clone,Deserialize)]
pub struct Identity{
    pub user_id:Uuid,
    pub Roles:Vec<Permission>,
    pub Permissions:Option<Role>,
    pub PermissionCodes:Vec<String>,
}

#[derive(Serialize,Debug,Clone,Deserialize)]
pub struct Permission{
    pub permission_id: Uuid,
    pub permission_code: String,
    pub permission_name :String,
    pub description: String,
    pub enabled:bool,
    pub create_time: chrono::DateTime<Local>,
    pub update_time: chrono::DateTime<Local>,
    pub data: Option<String>,
}

#[derive(Serialize,Debug,Clone,Deserialize)]
pub struct Role{
    pub role_id: Uuid,
    pub role_code: String,
    pub role_name:String,

    pub permissions:String,
    pub description:String,
    pub enabled:bool,
    pub create_time: chrono::DateTime<Local>,
    pub update_time: chrono::DateTime<Local>,
    pub data: Option<String>,
}
