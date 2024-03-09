use std::{ops::Deref, sync::Arc};

use async_trait::async_trait;
use bb8::ManageConnection;

use super::{
    backend::{AsyncBackend, Error},
    conn_pool::AsyncConnectionPool,
    object_pool::AsyncObjectPool,
};

#[derive(Clone)]
pub struct AsyncDatabasePool<B>(Arc<AsyncObjectPool<AsyncConnectionPool<B>>>)
where
    B: AsyncBackend;

impl<B> Deref for AsyncDatabasePool<B>
where
    B: AsyncBackend,
{
    type Target = Arc<AsyncObjectPool<AsyncConnectionPool<B>>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[async_trait]
pub trait AsyncDatabasePoolBuilder: AsyncBackend {
    async fn create_database_pool(
        self,
    ) -> Result<
        AsyncDatabasePool<Self>,
        Error<
            <Self::ConnectionManager as ManageConnection>::Error,
            Self::ConnectionError,
            Self::QueryError,
        >,
    > {
        self.init().await?;
        let backend = Arc::new(self);
        let object_pool = Arc::new(AsyncObjectPool::new(
            move || {
                let backend = backend.clone();
                Box::pin(async {
                    AsyncConnectionPool::new(backend)
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
        Ok(AsyncDatabasePool(object_pool))
    }
}

impl<AB> AsyncDatabasePoolBuilder for AB where AB: AsyncBackend {}
