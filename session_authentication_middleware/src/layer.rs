use axum_session_middleware::database_pool::AxumDatabasePool;
use chrono::{Duration, Utc};
use serde::{de::DeserializeOwned, Serialize};
use std::{fmt, hash::Hash, marker::PhantomData};
use tower_layer::Layer;

use crate::{AuthSessionService, Authentication};


/// Layer used to generate an AuthSessionService.
///
#[derive(Clone, Debug)]
pub struct AuthSessionLayer<P,User>
where
    P: AxumDatabasePool + Clone + fmt::Debug + Sync + Send + 'static,
    User:Authentication<User>,
{
    pub phantom_user: PhantomData<User>,
    pub phantom_pool: PhantomData<P>,
    // pub phantom_type: PhantomData<Type>,
}

impl<P,User> AuthSessionLayer<P,User>
where
    P: AxumDatabasePool + Clone + fmt::Debug + Sync + Send + 'static,
    User:Authentication<User>,
{   
    //TODO new phantom_type??
    pub fn new() -> Self {
        AuthSessionLayer{
            phantom_user: PhantomData::default(),
            phantom_pool: PhantomData::default(),
        }
        
    }
}

impl<S,P,User> Layer<S> for AuthSessionLayer<P,User>
where
    P: AxumDatabasePool + Clone + fmt::Debug + Sync + Send + 'static,
    User:Authentication<User>,
{
    type Service = AuthSessionService<S,P,User>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthSessionService {
            inner,
            phantom_session: PhantomData::default(),
            phantom_user: PhantomData::default(),
        }
    }
}
