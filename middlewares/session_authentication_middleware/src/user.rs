use chrono::Local;
use serde::Serialize;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Serialize,Debug,Clone,Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Identity{
    pub user_id:Uuid,

    pub roles:Vec<Role>,

    pub permissions:Vec<Permission>,

    pub permission_codes:Vec<String>,
}

#[derive(Serialize,Debug,Clone,Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Permission{
    pub permission_id: Uuid,

    pub permission_code: String,

    pub permission_name :String,

    pub description: String,
    
    #[serde(skip)]
    pub enabled:bool,
    
    pub create_time: chrono::DateTime<Local>,

    pub update_time: chrono::DateTime<Local>,

    #[serde(skip)]
    pub data: Option<String>,
}

#[derive(Serialize,Debug,Clone,Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Role{
    pub role_id: Uuid,
    
    pub role_code: String,
    
    pub role_name:String,

    pub permissions:String,

    pub description:String,

    #[serde(skip)]
    pub enabled:bool,

    pub create_time: chrono::DateTime<Local>,

    pub update_time: chrono::DateTime<Local>,

    #[serde(skip)]
    pub data: Option<String>,
}
