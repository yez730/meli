use chrono::Local;
use serde::Serialize;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Serialize,Debug,Clone,Deserialize)]
pub struct Identity{
    #[serde(rename ="userId")]
    pub user_id:Uuid,

    pub roles:Vec<Role>,

    pub permissions:Vec<Permission>,

    #[serde(rename ="permissionCodes")]
    pub permission_codes:Vec<String>,
}

#[derive(Serialize,Debug,Clone,Deserialize)]
pub struct Permission{
    #[serde(rename ="permissionId")]
    pub permission_id: Uuid,

    #[serde(rename ="permissionCode")]
    pub permission_code: String,

    #[serde(rename ="permissionName")]
    pub permission_name :String,

    pub description: String,
    
    #[serde(skip)]
    pub enabled:bool,
    
    #[serde(rename ="createTime")]
    pub create_time: chrono::DateTime<Local>,

    #[serde(rename ="updateTime")]
    pub update_time: chrono::DateTime<Local>,

    #[serde(skip)]
    pub data: Option<String>,
}

#[derive(Serialize,Debug,Clone,Deserialize)]
pub struct Role{
    #[serde(rename ="roleId")]
    pub role_id: Uuid,
    
    #[serde(rename ="roleCode")]
    pub role_code: String,
    
    #[serde(rename ="roleName")]
    pub role_name:String,

    pub permissions:String,

    pub description:String,

    #[serde(skip)]
    pub enabled:bool,

    #[serde(rename ="createTime")]
    pub create_time: chrono::DateTime<Local>,

    #[serde(rename ="updateTime")]
    pub update_time: chrono::DateTime<Local>,

    #[serde(skip)]
    pub data: Option<String>,
}
