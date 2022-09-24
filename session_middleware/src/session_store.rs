use std::{sync::Arc, collections::HashMap};

use anyhow::Ok;
use chrono::{DateTime, Local};
use dashmap::DashMap;
use futures::future::ok;
use uuid::Uuid;
use std::{
    fmt::Debug,
};
use crate::{session_data::AxumSessionData, database_pool::{AxumDatabasePool, SessionData}, config::AxumSessionConfig, session::SessionId};

#[derive(Clone, Debug)]
pub struct AxumSessionStore<T>
where 
    T:AxumDatabasePool + Clone + Debug + Sync + Send + 'static,
{
    // 内存中的所有的临时有效用户
    pub(crate) memory_store: Arc<DashMap<Uuid, AxumSessionData>>,
    pub(crate) database_store:T,
    pub(crate) config:AxumSessionConfig,
}


impl<T> AxumSessionStore<T>
where
    T: AxumDatabasePool + Clone + Debug + Sync + Send + 'static,
{
    #[inline]
    pub fn new(database: T) -> Self {
        Self {
            database_store:database,
            memory_store: Default::default(),
            config: Default::default(),
        }
    }
    pub fn with_config(&mut self,config: AxumSessionConfig){
        self.config=config;
    }
   
    pub(crate) async fn store(
        &self,
        session_data:&AxumSessionData
    ) -> Result<(), anyhow::Error> {
        let sess=session_data.clone();
        if let Some(user_id)=sess.user_id {
            let session_data=SessionData{
                session_id:sess.session_id,
                user_id:user_id,
                init_time:sess.init_time,
                expiry_time:sess.expiry_time,
                data:sess.data,
            };

            self.database_store.store(&session_data).await?;
        } else{
            self.memory_store.insert(session_data.session_id, session_data.clone());
        }

        Ok(())
    }

    pub(crate) async fn load_or_init(&self, session_id: &Uuid) -> Result<Option<AxumSessionData>, anyhow::Error>{
        if let Some(session_data)=self.database_store.load(&session_id).await?{
                let sess=AxumSessionData{
                    session_id:session_data.session_id,
                    user_id:Some(session_data.user_id),
                    init_time:session_data.init_time,
                    expiry_time:session_data.expiry_time,
                    data:session_data.data
                };
                return Ok(Some(sess));
        } else {
            match self.memory_store.get(&session_id) {
                Some(s)=>{
                    return Ok(Some(s.clone()));
                }
                None=>{
                    let sess=AxumSessionData::init(session_id.clone(),self.config.memory_clear_timeout);
                    
                    //TODO 膨胀
                    self.memory_store.insert(sess.session_id, sess.clone());
                    return Ok(Some(sess));
                }
            };
        }
    }
}
