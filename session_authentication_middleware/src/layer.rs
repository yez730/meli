use std::{fmt, marker::PhantomData};
use axum_session_middleware::database::AxumDatabaseTrait;
use tower_layer::Layer;

use crate::{session::Authentication, service::AuthSessionService};

#[derive(Clone, Debug)]
pub struct AuthSessionLayer<SessionDB,AuthDB,User,>
where
    AuthDB: Clone + Send + Sync + fmt::Debug + 'static,
    User:Authentication<User,AuthDB> + Clone + Send + Sync + 'static,
    SessionDB: AxumDatabaseTrait + Clone + fmt::Debug + Sync + Send + 'static,
{
    pub(crate) database:AuthDB,
    pub phantom_user: PhantomData<User>,
    pub phantom_session_db: PhantomData<SessionDB>,
}

impl<AuthDB,User,SessionDB> AuthSessionLayer<SessionDB,AuthDB,User,>
where
AuthDB: Clone + Send + Sync + fmt::Debug + 'static,
User:Authentication<User,AuthDB> + Clone + Send + Sync + 'static,
SessionDB: AxumDatabaseTrait + Clone + fmt::Debug + Sync + Send + 'static,
{   
    pub fn new(database:AuthDB) -> Self {
        AuthSessionLayer{
            database,
            phantom_user: PhantomData::default(),
            phantom_session_db: PhantomData::default(),
        }
    }
}

impl<S,AuthDB,User,SessionDB> Layer<S> for AuthSessionLayer<SessionDB,AuthDB,User,>
where
    AuthDB: Clone + Send + Sync + fmt::Debug + 'static,
    User:Authentication<User,AuthDB> + Clone + Send + Sync + 'static,
    SessionDB: AxumDatabaseTrait + Clone + fmt::Debug + Sync + Send + 'static,
{
    type Service = AuthSessionService<S,AuthDB,User,SessionDB>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthSessionService {
            database:self.database.clone(),
            inner,
            phantom_user: PhantomData::default(),
            phantom_session_db: PhantomData::default(),
        }
    }
}
