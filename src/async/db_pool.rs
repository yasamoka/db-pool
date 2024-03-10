use std::{ops::Deref, sync::Arc};

use async_trait::async_trait;
use bb8::ManageConnection;

use super::{
    backend::{r#trait::Backend, Error},
    conn_pool::ConnectionPool,
    object_pool::ObjectPool,
};

#[derive(Clone)]
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

#[async_trait]
pub trait DatabasePoolBuilder: Backend {
    async fn create_database_pool(
        self,
    ) -> Result<
        DatabasePool<Self>,
        Error<
            <Self::ConnectionManager as ManageConnection>::Error,
            Self::ConnectionError,
            Self::QueryError,
        >,
    > {
        self.init().await?;
        let backend = Arc::new(self);
        let object_pool = Arc::new(ObjectPool::new(
            move || {
                let backend = backend.clone();
                Box::pin(async {
                    ConnectionPool::new(backend)
                        .await
                        .expect("connection pool creation must succeed")
                })
            },
            |mut conn_pool| {
                Box::pin(async {
                    conn_pool
                        .clean()
                        .await
                        .expect("connection pool cleaning must succeed");
                    conn_pool
                })
            },
        ));
        Ok(DatabasePool(object_pool))
    }
}

impl<AB> DatabasePoolBuilder for AB where AB: Backend {}
