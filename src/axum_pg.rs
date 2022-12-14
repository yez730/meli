use std::collections::HashMap;

use anyhow::{anyhow, Ok};
use async_trait::async_trait;
use axum_session_middleware::database::{AxumDatabaseTrait,self};
use chrono::Local;
use uuid::Uuid;
use crate::models::{NewSession, Session};
use crate::schema::*;

use diesel::PgConnection;
use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use diesel::r2d2::Pool;

#[derive(Clone)]
pub struct AxumPg{
    pub pool:Pool<ConnectionManager<PgConnection>>,
}

impl std::fmt::Debug for AxumPg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AxumPgPool")
         .field("connection", &"`&self.connection`") //TODO fix
         .finish()
    }
}

#[async_trait]
impl AxumDatabaseTrait for AxumPg{
    async fn store(&self,session_data:&database::SessionData) -> Result<(), anyhow::Error>{
        let mut conn=self.pool.get()
            .map_err(|e| anyhow!("Get connection error: {}",e))?;
        
        let session=sessions::table
            .filter(sessions::session_id.eq(session_data.session_id))
            .get_result::<Session>(&mut *conn).ok();
        
        let data_str=serde_json::to_string(&session_data.data).map_err(|e| anyhow!("Serialize data error: {}",e))?;
        match session {
            Some(session)=>{
                diesel::update(sessions::table.find(session.id))
                .set((
                        sessions::user_id.eq(session_data.user_id), // TODO need？
                        sessions::data.eq(data_str),
                        sessions::expiry_time.eq(session_data.expiry_time),
                        sessions::update_time.eq(Local::now()),
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
                    data: data_str.as_str(),
                };
                diesel::insert_into(sessions::table).values(&new_session)
                    .execute(&mut *conn)
                    .map_err(|e| anyhow!("Execute error: {}",e))?;
            }
        }       

        Ok(())
    }

    async fn load(&self, session_id: &Uuid) -> Result<database::SessionData, anyhow::Error>{
        let mut conn=self.pool.get().unwrap();
        
        sessions::table
            .filter(sessions::session_id.eq(session_id))
            .get_result::<Session>(&mut *conn)
            .map(|session|{
                let data=serde_json::from_str::<HashMap<String,String>>(&session.data).unwrap();
                
                database::SessionData{
                    session_id:session.session_id,
                    user_id:session.user_id,
                    init_time:session.init_time,
                    expiry_time:session.expiry_time,
                    data,
                }
            })
            .map_err(|e|anyhow!("Execute error: {}",e))
    }
}
