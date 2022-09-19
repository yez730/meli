use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axum_database_sessions::{AxumDatabasePool, SessionError};
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
    async fn initiate(&self, _table_name: &str) -> Result<(), SessionError> {
        Ok(())
    }

    async fn delete_by_expiry(&self, _table_name: &str) -> Result<(), SessionError> {
        let mut conn=self.connection.lock().map_err(|e| SessionError::GenericNotSupportedError(e.to_string()))?;
        let _num_deleted=diesel::delete(sessions::dsl::sessions.filter(sessions::dsl::expiry_time.lt(now)))
            .execute(&mut *conn).map_err(|e|SessionError::GenericNotSupportedError(e.to_string()))?;

        Ok(())
    }

    async fn count(&self, _table_name: &str) -> Result<i64, SessionError> {
        let mut conn=self.connection.lock().map_err(|e| SessionError::GenericNotSupportedError(e.to_string()))?;
        let count=sessions::dsl::sessions.count()
            .execute(&mut *conn)
            .map_err(|e|SessionError::GenericNotSupportedError(e.to_string()))?;

        return Ok(count as i64);
    }

    async fn store(
        &self,
        uuid: &str,
        data: &str,
        expires: i64,
        _table_name: &str,
    ) -> Result<(), SessionError> {
        let mut conn=self.connection.lock().map_err(|e| SessionError::GenericNotSupportedError(e.to_string()))?;
        
        let uuid=Uuid::parse_str(uuid).unwrap(); //TODO fix unwrap
        
        let session=sessions::dsl::sessions
            .filter(sessions::dsl::session_id.eq(uuid))
            .get_result::<Session>(&mut *conn).ok();

        //重新设置 session.data 内部时间字段的时区
        let mut v:Value=serde_json::from_str(&data)?;
        v["expires"]=json!(v["expires"].as_str().unwrap().parse::<DateTime<Utc>>().unwrap().with_timezone(&Local)); //TODO fix unwrap
        v["autoremove"]=json!(v["autoremove"].as_str().unwrap().parse::<DateTime<Utc>>().unwrap().with_timezone(&Local));
        let data=v.to_string();
        
        match session {
            Some(session)=>{
                diesel::update(sessions::dsl::sessions.find(session.id))
                .set((
                        sessions::dsl::expiry_time.eq(Utc.timestamp(expires,0)),
                        sessions::dsl::data.eq(data),
                        sessions::dsl::update_time.eq(Local::now()),
                    ))
                .execute(&mut *conn)
                .map_err(|e|SessionError::GenericNotSupportedError(e.to_string()))?;
            }
            None=>{
                let new_session=NewSession{
                    session_id: &uuid,
                    expiry_time: Utc.timestamp(expires,0).with_timezone(&Local),
                    data: &data,
                    create_time: Local::now(),
                    update_time: Local::now(),
                };
                diesel::insert_into(sessions::table).values(&new_session)
                    .execute(&mut *conn).map_err(|e|SessionError::GenericNotSupportedError(e.to_string()))?;
            }
        }       

        Ok(())
    }

    async fn load(&self, uuid: &str, _table_name: &str) -> Result<Option<String>, SessionError> {
        let mut conn=self.connection.lock().map_err(|e| SessionError::GenericNotSupportedError(e.to_string()))?;
        
        let uuid=Uuid::parse_str(uuid).unwrap();

        let session=sessions::dsl::sessions
            .filter(sessions::dsl::session_id.eq(uuid)) 
            .filter(sessions::dsl::expiry_time.gt(now))
            .get_result::<Session>(&mut *conn)
            .ok()
            .map(|s|s.data);

        Ok(session)
    }

    async fn delete_one_by_id(&self, uuid: &str, _table_name: &str) -> Result<(), SessionError> {
        let mut conn=self.connection.lock().map_err(|e| SessionError::GenericNotSupportedError(e.to_string()))?;
     
        let uuid=Uuid::parse_str(uuid).unwrap();
        
        diesel::delete(sessions::dsl::sessions.filter(sessions::dsl::session_id.eq(uuid)))
            .execute(&mut *conn).map_err(|e|SessionError::GenericNotSupportedError(e.to_string()))?;
       
        Ok(())
    }

    async fn delete_all(&self, _table_name: &str) -> Result<(), SessionError> {
        let mut conn=self.connection.lock().map_err(|e| SessionError::GenericNotSupportedError(e.to_string()))?;
     
        diesel::delete(sessions::dsl::sessions)
            .execute(&mut *conn).map_err(|e|SessionError::GenericNotSupportedError(e.to_string()))?;

        Ok(())
    }
}