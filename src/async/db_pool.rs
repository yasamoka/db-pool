use std::{future::Future, pin::Pin, sync::Arc};

use super::{
    backend::AsyncBackend, conn_pool::ReusableAsyncConnectionPool, object_pool::AsyncObjectPool,
};

pub trait AsyncDatabasePoolBuilder: AsyncBackend {
    fn create_database_pool(
        self,
    ) -> Arc<
        AsyncObjectPool<
            ReusableAsyncConnectionPool<Self>,
            impl Fn()
                -> Pin<Box<dyn Future<Output = ReusableAsyncConnectionPool<Self>> + Send + 'static>>,
            impl Fn(
                ReusableAsyncConnectionPool<Self>,
            )
                -> Pin<Box<dyn Future<Output = ReusableAsyncConnectionPool<Self>> + Send + 'static>>,
        >,
    > {
        let backend = Arc::new(self);

        Arc::new(AsyncObjectPool::new(
            move || {
                let backend = backend.clone();
                Box::pin(async { ReusableAsyncConnectionPool::new(backend).await })
            },
            |mut conn_pool| {
                Box::pin(async {
                    conn_pool.clean().await;
                    conn_pool
                })
            },
        ))
    }
}

impl<AB> AsyncDatabasePoolBuilder for AB where AB: AsyncBackend {}
