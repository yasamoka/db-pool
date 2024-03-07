use std::{ops::Deref, sync::Arc};

use super::{backend::AsyncBackend, conn_pool::AsyncConnectionPool, object_pool::AsyncObjectPool};

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

pub trait AsyncDatabasePoolBuilder: AsyncBackend {
    fn create_database_pool(self) -> AsyncDatabasePool<Self> {
        let backend = Arc::new(self);
        let object_pool = Arc::new(AsyncObjectPool::new(
            move || {
                let backend = backend.clone();
                Box::pin(async { AsyncConnectionPool::new(backend).await })
            },
            |mut conn_pool| {
                Box::pin(async {
                    conn_pool.clean().await;
                    conn_pool
                })
            },
        ));
        AsyncDatabasePool(object_pool)
    }
}

impl<AB> AsyncDatabasePoolBuilder for AB where AB: AsyncBackend {}
