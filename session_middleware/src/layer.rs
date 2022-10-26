use std::fmt;

use tower_layer::Layer;

use crate::{database::AxumDatabaseTrait, session_store::AxumSessionStore, service::AxumSessionService};

#[derive(Clone)]
pub struct AxumSessionLayer<T>
where
    T: AxumDatabaseTrait + Clone + fmt::Debug + std::marker::Sync + std::marker::Send + 'static,
{
    session_store: AxumSessionStore<T>,
}

impl<T> AxumSessionLayer<T>
where
    T: AxumDatabaseTrait + Clone + fmt::Debug + std::marker::Sync + std::marker::Send + 'static,
{
    #[inline]
    pub fn new(session_store: AxumSessionStore<T>) -> Self {
        AxumSessionLayer { session_store }
    }
}

impl<S, T> Layer<S> for AxumSessionLayer<T>
where
    T: AxumDatabaseTrait + Clone + fmt::Debug + std::marker::Sync + std::marker::Send + 'static,
{
    type Service = AxumSessionService<S, T>;

    fn layer(&self, inner: S) -> Self::Service {
        AxumSessionService {
            session_store: self.session_store.clone(),
            inner,
        }
    }
}
