use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Ok};
use async_trait::async_trait;
use axum_session_middleware::database_pool::{AxumDatabasePool,self};
use chrono::DateTime;
use chrono::{Local, Utc,offset::TimeZone};
use serde_json::{Value, json};
use uuid::Uuid;
use crate::models::{NewSession, Session};
use crate::schema::*;

use diesel::dsl::now;
use diesel::PgConnection;
use diesel::prelude::*;

#[derive(Clone)]
pub struct AxumPgPool{
    pub connection:Arc<Mutex<PgConnection>>,
}

impl std::fmt::Debug for AxumPgPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AxumPgPool")
         .field("connection", &"`&self.connection`") //TODO fix
         .finish()
    }
}

#[async_trait]
impl AxumDatabasePool for AxumPgPool{
    async fn store(&self,session_data:&database_pool::SessionData) -> Result<(), anyhow::Error>{
        let mut conn=self.connection.lock()
            .map_err(|e| anyhow!("Get connection error: {}",e))?;
        
        let session=sessions::dsl::sessions
            .filter(sessions::dsl::session_id.eq(session_data.session_id))
            .get_result::<Session>(&mut *conn).ok();
        
        let data_str=serde_json::to_string(&session_data.data).map_err(|e| anyhow!("Serialize data error: {}",e))?;
        match session {
            Some(session)=>{
                diesel::update(sessions::dsl::sessions.find(session.id))
                .set((
                        sessions::dsl::user_id.eq(session_data.user_id), // TODO need？
                        sessions::dsl::data.eq(data_str),
                        sessions::dsl::expiry_time.eq(session_data.expiry_time),
                        sessions::dsl::update_time.eq(Local::now()),
                    ))
                .execute(&mut *conn)
                .map_err(|e| anyhow!("Execute error: {}",e))?;
            }
            None=>{
                let new_session=NewSession{
                    session_id: &session_data.session_id,
                    user_id: &session_data.user_id,
                    expiry_time: session_data.expiry_time,
                    init_time: session_data.init_time,
                    create_time: Local::now(),
                    update_time: Local::now(),
                    data: Some(data_str.as_str()),
                };
                diesel::insert_into(sessions::table).values(&new_session)
                    .execute(&mut *conn)
                    .map_err(|e| anyhow!("Execute error: {}",e))?;
            }
        }       

        Ok(())
    }

    async fn load(&self, session_id: &Uuid) -> Result<database_pool::SessionData, anyhow::Error>{
        use std::result::Result::Ok;
        let mut conn=self.connection.lock()
            .map_err(|e| anyhow!("Get connection error: {}",e))?;
        
        let session=sessions::dsl::sessions
            .filter(sessions::dsl::session_id.eq(session_id)) 
            .get_result::<Session>(&mut *conn);

        match session {
            Err(e)=>Err(anyhow!("Get connection error: {}",e)),
            Ok(session)=>{
                let data=match session.data {
                    Some(data) if serde_json::from_str::<HashMap<String,String>>(&data).is_ok()
                        =>serde_json::from_str::<HashMap<String,String>>(&data).unwrap(),
                    _=>Default::default(),
                };
                let session_data=database_pool::SessionData{
                    session_id:session.session_id,
                    user_id:session.user_id,
                    init_time:session.init_time,
                    expiry_time:session.expiry_time,
                    data:data,
                };
                Ok(session_data)
            }
        }
    }
}
