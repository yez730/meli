use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axum_database_sessions::{AxumDatabasePool, SessionError};
use chrono::DateTime;
use chrono::{Local, Utc,offset::TimeZone};
use serde_json::{Value, json};
use uuid::Uuid;
use crate::models::{NewSession, Session};
use crate::schema::{sessions::dsl::*,sessions}; // sessions for sessions.table

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
    async fn initiate(&self, _table_name: &str) -> Result<(), SessionError> {
        Ok(())
    }

    async fn delete_by_expiry(&self, _table_name: &str) -> Result<(), SessionError> {
        let mut conn=self.connection.lock().map_err(|e| SessionError::GenericNotSupportedError(e.to_string()))?;
        let _num_deleted=diesel::delete(sessions.filter(expiry_time.lt(now)))
            .execute(&mut *conn).map_err(|e|SessionError::GenericNotSupportedError(e.to_string()))?;

        Ok(())
    }

    async fn count(&self, _table_name: &str) -> Result<i64, SessionError> {
        let mut conn=self.connection.lock().map_err(|e| SessionError::GenericNotSupportedError(e.to_string()))?;
        let count=sessions.count()
            .execute(&mut *conn)
            .map_err(|e|SessionError::GenericNotSupportedError(e.to_string()))?;

        return Ok(count as i64);
    }

    async fn store(
        &self,
        s_uuid: &str,
        s_data: &str,
        expires: i64,
        _table_name: &str,
    ) -> Result<(), SessionError> {
        let mut conn=self.connection.lock().map_err(|e| SessionError::GenericNotSupportedError(e.to_string()))?;
        
        let s_uuid=Uuid::parse_str(s_uuid).unwrap(); //TODO fix unwrap
        
        let session=sessions
            .filter(session_id.eq(s_uuid))
            .get_result::<Session>(&mut *conn).ok();

        //重新设置 session.data 内部时间字段的时区
        let mut v:Value=serde_json::from_str(&s_data)?;
        v["expires"]=json!(v["expires"].as_str().unwrap().parse::<DateTime<Utc>>().unwrap().with_timezone(&Local)); //TODO fix unwrap
        v["autoremove"]=json!(v["autoremove"].as_str().unwrap().parse::<DateTime<Utc>>().unwrap().with_timezone(&Local));
        let s_data=v.to_string();
        
        match session {
            Some(session)=>{
                diesel::update(sessions.find(session.id))
                .set((
                        expiry_time.eq(Utc.timestamp(expires,0)),
                        data.eq(s_data),
                        update_time.eq(Local::now()),
                    ))
                .execute(&mut *conn)
                .map_err(|e|SessionError::GenericNotSupportedError(e.to_string()))?;
            }
            None=>{
                let new_session=NewSession{
                    session_id: &s_uuid,
                    expiry_time: Utc.timestamp(expires,0).with_timezone(&Local),
                    data: &s_data,
                    create_time: Local::now(),
                    update_time: Local::now(),
                };
                diesel::insert_into(sessions::table).values(&new_session)
                    .execute(&mut *conn).map_err(|e|SessionError::GenericNotSupportedError(e.to_string()))?;
            }
        }       

        Ok(())
    }

    async fn load(&self, s_uuid: &str, _table_name: &str) -> Result<Option<String>, SessionError> {
        let mut conn=self.connection.lock().map_err(|e| SessionError::GenericNotSupportedError(e.to_string()))?;
        
        let s_uuid=Uuid::parse_str(s_uuid).unwrap();

        let session=sessions
            .filter(session_id.eq(s_uuid)) 
            .filter(expiry_time.gt(now))
            .get_result::<Session>(&mut *conn)
            .ok()
            .map(|s|s.data);

        Ok(session)
    }

    async fn delete_one_by_id(&self, s_uuid: &str, _table_name: &str) -> Result<(), SessionError> {
        let mut conn=self.connection.lock().map_err(|e| SessionError::GenericNotSupportedError(e.to_string()))?;
     
        let s_uuid=Uuid::parse_str(s_uuid).unwrap();
        
        let _num_deleted=diesel::delete(sessions.filter(session_id.eq(s_uuid)))
            .execute(&mut *conn).map_err(|e|SessionError::GenericNotSupportedError(e.to_string()))?;
       
        Ok(())
    }

    async fn delete_all(&self, _table_name: &str) -> Result<(), SessionError> {
        let mut conn=self.connection.lock().map_err(|e| SessionError::GenericNotSupportedError(e.to_string()))?;
     
        let _num_deleted=diesel::delete(sessions)
            .execute(&mut *conn).map_err(|e|SessionError::GenericNotSupportedError(e.to_string()))?;

        Ok(())
    }
}