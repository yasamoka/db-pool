use std::{ops::Deref, sync::Arc};

use super::{
    backend::{r#trait::Backend, Error},
    conn_pool::ConnectionPool,
    object_pool::ObjectPool,
};

pub struct DatabasePool<B>(Arc<ObjectPool<ConnectionPool<B>>>)
where
    B: Backend;

impl<B> Deref for DatabasePool<B>
where
    B: Backend,
{
    type Target = Arc<ObjectPool<ConnectionPool<B>>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub trait DatabasePoolBuilder: Backend {
    fn create_database_pool(
        self,
    ) -> Result<DatabasePool<Self>, Error<Self::ConnectionError, Self::QueryError>> {
        self.init()?;
        let backend = Arc::new(self);
        let object_pool = Arc::new(ObjectPool::new(
            move || {
                let backend = backend.clone();
                ConnectionPool::new(backend).expect("connection pool creation must succeed")
            },
            |conn_pool| {
                conn_pool
                    .clean()
                    .expect("connection pool cleaning must succeed");
            },
        ));
        Ok(DatabasePool(object_pool))
    }
}

impl<B> DatabasePoolBuilder for B where B: Backend + Sized {}
