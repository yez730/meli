use std::{sync::Arc};

use anyhow::Ok;
use dashmap::DashMap;
use uuid::Uuid;
use std::{
    fmt::Debug,
};
use crate::{session_data::AxumSessionData, database::{AxumDatabaseTrait, SessionData}, config::AxumSessionConfig};

#[derive(Clone, Debug)]
pub struct AxumSessionStore<T>
where 
    T:AxumDatabaseTrait + Clone + Debug + Sync + Send + 'static,
{
    // 内存中的所有的临时有效用户
    pub(crate) memory_store: Arc<DashMap<Uuid, AxumSessionData>>,
    pub(crate) database:T,
    pub(crate) config:AxumSessionConfig,
}

impl<T> AxumSessionStore<T>
where
    T: AxumDatabaseTrait + Clone + Debug + Sync + Send + 'static,
{
    #[inline]
    pub fn new(database: T,config:AxumSessionConfig) -> Self {
        Self {
            database,
            memory_store: Default::default(),
            config,
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
                user_id,
                init_time:sess.init_time,
                expiry_time:sess.expiry_time,
                data:sess.data,
            };
            self.database.store(&session_data).await?;
        } else{
            self.memory_store.insert(session_data.session_id, session_data.clone());
        }

        Ok(())
    }

    pub(crate) async fn load_or_init(&self, session_id: &Uuid) -> Option<AxumSessionData>{
        use std::result::Result::Ok;
        
        match self.database.load(&session_id).await {
            Ok(session_data)=>{
                let sess=AxumSessionData{
                    session_id:session_data.session_id,
                    user_id:Some(session_data.user_id),
                    init_time:session_data.init_time,
                    expiry_time:session_data.expiry_time,
                    data:session_data.data
                };
                return Some(sess);
            }
            Err(_)=>{
                match self.memory_store.get(&session_id) {
                    Some(s)=>{
                        return Some(s.clone());
                    }
                    None=>{
                        let sess=AxumSessionData::init(session_id.clone(),self.config.memory_clear_timeout);
                        
                        //TODO 膨胀
                        self.memory_store.insert(sess.session_id, sess.clone());
                        return Some(sess);
                    }
                };
            }
        }
    }
}
