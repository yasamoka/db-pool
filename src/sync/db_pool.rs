use std::sync::Arc;

use super::{backend::Backend, conn_pool::ReusableConnectionPool, object_pool::ObjectPool};

pub trait DatabasePoolBuilder: Backend + Sized {
    fn create_database_pool(
        self,
    ) -> Arc<
        ObjectPool<
            ReusableConnectionPool<Self>,
            impl Fn() -> ReusableConnectionPool<Self>,
            impl Fn(&mut ReusableConnectionPool<Self>),
        >,
    > {
        let backend = Arc::new(self);

        Arc::new(ObjectPool::new(
            move || {
                let backend = backend.clone();
                ReusableConnectionPool::new(backend)
            },
            |conn_pool| conn_pool.clean(),
        ))
    }
}

impl<B> DatabasePoolBuilder for B where B: Backend + Sized {}
