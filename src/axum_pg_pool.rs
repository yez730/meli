use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axum_database_sessions::{AxumDatabasePool, SessionError};
use chrono::{Local, Utc,offset::TimeZone};
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
        s_id: &str,
        session: &str,
        expires: i64,
        _table_name: &str,
    ) -> Result<(), SessionError> {
        let mut conn=self.connection.lock().map_err(|e| SessionError::GenericNotSupportedError(e.to_string()))?;
        let s_uuid=Uuid::parse_str(s_id).map_err(|e|SessionError::GenericSelectError(e.to_string()))?;
        let sesses=sessions
            .filter(session_id.eq(s_uuid))
            .limit(1)
            .load::<Session>(&mut *conn).map_err(|e|SessionError::GenericNotSupportedError(e.to_string()))?;

        if sesses.len()==0 {
            let new_session=NewSession{
                session_id: s_uuid,
                expiry_time: Local::now(),
                extra: session,
            };

            

            diesel::insert_into(sessions::table).values(&new_session)
                .execute(&mut *conn).map_err(|e|SessionError::GenericNotSupportedError(e.to_string()))?;
        } else {
            diesel::update(sessions.find(sesses[0].id))
                .set((expiry_time.eq(Utc.timestamp(expires,0)),
                extra.eq(session)))
                .execute(&mut *conn)
                .map_err(|e|SessionError::GenericNotSupportedError(e.to_string()))?;
        }
   
        Ok(())
    }

    async fn load(&self, s_id: &str, _table_name: &str) -> Result<Option<String>, SessionError> {
        let mut conn=self.connection.lock().map_err(|e| SessionError::GenericNotSupportedError(e.to_string()))?;
        let s_uuid=Uuid::parse_str(s_id).map_err(|e|SessionError::GenericSelectError(e.to_string()))?;
        let sesses=sessions
            .filter(session_id.eq(s_uuid))
            .filter(expiry_time.lt(now))
            .limit(1)
            .load::<Session>(&mut *conn).map_err(|e|SessionError::GenericNotSupportedError(e.to_string()))?;

        if sesses.len()==0 {
            return Err(SessionError::GenericNotSupportedError("Unexcepted error".to_string()))
        }

        Ok(Some(sesses[0].extra.clone())) //TODO 
    }

    async fn delete_one_by_id(&self, s_id: &str, _table_name: &str) -> Result<(), SessionError> {
        let mut conn=self.connection.lock().map_err(|e| SessionError::GenericNotSupportedError(e.to_string()))?;
     
        let s_uuid=Uuid::parse_str(s_id).map_err(|e|SessionError::GenericSelectError(e.to_string()))?;
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