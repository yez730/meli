use anyhow::Ok;
use chrono::Local;
use uuid::Uuid;
use std::{
    fmt::Debug,
};

use crate::{session_data::AxumSessionData, database_pool::AxumDatabasePool, session_store::AxumSessionStore, constants::{ SESSIONID, SessionKeys}};

#[derive(Clone,Debug)]
pub struct AxumSession<T>
where
    T: AxumDatabasePool + Clone + Debug + Sync + Send + 'static,
{
    pub store: AxumSessionStore<T>,
    pub(crate) session_id:SessionId,
    pub(crate) session_data:AxumSessionData,
    pub(crate) is_modified:bool,
}

//TODO viriant as str
pub(crate) enum SessionIdType{
    Cached,
    Empty
}

#[derive(Clone,Debug)]
pub(crate) struct SessionId(pub(crate) String);// `{type}+uuid`, c:cached, e:empty
impl SessionId{
    pub(crate) fn from(s: &str) -> Self{
        let ty_add=s.get(0..2).and_then(|ty| (ty=="c+"||ty=="e+").then_some(ty));
        let uuid=s.get(2..).and_then(|uuid|Uuid::parse_str(uuid).ok());
        
        if ty_add.is_none() || uuid.is_none(){
            return SessionId::init_session_id();
        }

        SessionId(s.to_string())
    }

    pub(crate) fn init_session_id()->Self{
        SessionId(format!("e+{}",Uuid::new_v4()))
    }

    pub(crate) fn get_session_id_type(&self)->SessionIdType{
        let ty=self.0.get(0..1).and_then(|ty| (ty=="c"||ty=="e").then_some(ty));
        match ty {
            Some("c")=>SessionIdType::Cached,
            _=>SessionIdType::Empty,
        }
    }

    pub(crate) fn get_session_guid(&self)->Uuid{
        self.0.get(2..)
            .and_then(|uuid|
                Uuid::parse_str(uuid).ok()
            )
            .unwrap()
    }

    pub(crate) fn change_session_id_cached_type(&mut self){
        *self=SessionId(format!("c+{}",self.get_session_guid()));
    }
}

impl<T> AxumSession<T>
where
    T: AxumDatabasePool + Clone + Debug + Sync + Send + 'static,
{
    pub(crate) async fn load_or_init(store: &AxumSessionStore<T>,session_id:Option<&str>)->Result<AxumSession<T>,anyhow::Error>{
        match session_id {
            None=>{
                let session_id=SessionId::init_session_id();
                Ok(AxumSession{
                    session_id:session_id.clone(),
                    store:store.clone(),
                    session_data:AxumSessionData::init(session_id.get_session_guid(), store.config.memory_clear_timeout),
                    is_modified:false,
                })
            }
            Some(session_id)=>{
                let session_id=SessionId::from(session_id);
                if matches!(session_id.get_session_id_type(),SessionIdType::Empty){
                    let session_id=SessionId::init_session_id();
                    return Ok(AxumSession{
                        session_id:session_id.clone(),
                        store:store.clone(),
                        session_data:AxumSessionData::init(session_id.get_session_guid(), store.config.memory_clear_timeout),
                        is_modified:false,
                    });
                }

                match store.load_or_init(&session_id.get_session_guid()).await? {
                    Some(sess) if sess.expiry_time>Local::now()=>{                       
                        Ok(AxumSession{
                            session_id,
                            store:store.clone(),
                            session_data:sess,
                            is_modified:false,
                        })
                    }
                    _=>{
                        let session_id=SessionId::init_session_id();

                        Ok(AxumSession{
                            session_id:session_id.clone(),
                            store:store.clone(),
                            session_data:AxumSessionData::init(session_id.get_session_guid(), store.config.memory_clear_timeout),
                            is_modified:false,
                        })
                    }
                }
            }
        }
	}

	pub fn set_user_id(&mut self,user_id:Uuid){
        self.session_data.user_id=Some(user_id);

		self.is_modified=true;
        tracing::error!("is_modified {}",self.is_modified);
        self.session_id.change_session_id_cached_type();
        tracing::error!("session_id {}",self.session_id.0)
	}

    pub fn get_logined_user_id(&self)->Option<Uuid>{
        self.session_data.user_id
    }

    pub fn get_identity_str(&self)->&str{
        self.session_data.data[SessionKeys::Identity].as_str()
    }

    pub fn set_data(&mut self,key:String,val:String){
        self.session_data.data.insert(key, val);

		self.is_modified=true;
        self.session_id.change_session_id_cached_type();
    }

    pub fn clear(&mut self){
        let session_id=SessionId::init_session_id();
        *self=AxumSession{
            session_id:session_id.clone(),
            store:self.store.clone(),
            session_data:AxumSessionData::init(session_id.get_session_guid(), self.store.config.memory_clear_timeout),
            is_modified:false,
        };

        //TODO is_modified= true;
    }

	pub(crate) async fn commit(&mut self)->Result<(),anyhow::Error>{
        tracing::error!("begin commit is_modified:{}",self.is_modified);
        if !self.is_modified{
            return Ok(());
        }

        if self.session_data.expiry_time>Local::now(){
            if self.session_data.user_id.is_none(){
                self.session_data.expiry_time=Local::now()+self.store.config.memory_clear_timeout;
            }else{
                self.session_data.expiry_time=Local::now()+self.store.config.idle_timeout;
            }
        }
        tracing::error!("begin store:{}",self.is_modified);
        self.store.store(&self.session_data).await?;
        self.is_modified=false;

        Ok(())
	}
}