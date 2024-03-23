use std::ops::Deref;

use r2d2::Pool;

use super::{backend::r#trait::Backend, conn_pool::ConnectionPool, object_pool::Reusable};

/// Connection pool wrapper to facilitate the use of pools in code under test and reusable pools in tests
pub enum PoolWrapper<B: Backend> {
    /// Connection pool used in code under test
    Pool(Pool<B::ConnectionManager>),
    /// Reusable connection pool used in tests
    ReusablePool(Reusable<'static, ConnectionPool<B>>),
}

impl<B: Backend> Deref for PoolWrapper<B> {
    type Target = Pool<B::ConnectionManager>;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Pool(pool) => pool,
            Self::ReusablePool(pool) => pool,
        }
    }
}
