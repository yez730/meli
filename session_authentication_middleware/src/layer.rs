use std::{fmt, marker::PhantomData};
use axum_session_middleware::database_pool::AxumDatabasePool;
use tower_layer::Layer;

use crate::{session::Authentication, service::AuthSessionService};

#[derive(Clone, Debug)]
pub struct AuthSessionLayer<SessionP,AuthP,User,>
where
    AuthP: Clone + Send + Sync + fmt::Debug + 'static,
    User:Authentication<User,AuthP> + Clone + Send + Sync + 'static,
    SessionP: AxumDatabasePool + Clone + fmt::Debug + Sync + Send + 'static,
{
    pub(crate) database_pool:AuthP,
    pub phantom_user: PhantomData<User>,
    pub phantom_session_pool: PhantomData<SessionP>,
}

impl<AuthP,User,SessionP> AuthSessionLayer<SessionP,AuthP,User,>
where
AuthP: Clone + Send + Sync + fmt::Debug + 'static,
User:Authentication<User,AuthP> + Clone + Send + Sync + 'static,
SessionP: AxumDatabasePool + Clone + fmt::Debug + Sync + Send + 'static,
{   
    //TODO new phantom_type??
    pub fn new(pool:AuthP) -> Self {
        AuthSessionLayer{
            database_pool:pool,
            phantom_user: PhantomData::default(),
            phantom_session_pool: PhantomData::default(),
        }
    }
}

impl<S,AuthP,User,SessionP> Layer<S> for AuthSessionLayer<SessionP,AuthP,User,>
where
    AuthP: Clone + Send + Sync + fmt::Debug + 'static,
    User:Authentication<User,AuthP> + Clone + Send + Sync + 'static,
    SessionP: AxumDatabasePool + Clone + fmt::Debug + Sync + Send + 'static,
{
    type Service = AuthSessionService<S,AuthP,User,SessionP>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthSessionService {
            database_pool:self.database_pool.clone(),
            inner,
            phantom_user: PhantomData::default(),
            phantom_session_pool: PhantomData::default(),
        }
    }
}
