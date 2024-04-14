use std::ops::Deref;

use r2d2::Pool;

use super::{
    backend::r#trait::Backend, conn_pool::SingleUseConnectionPool, db_pool::ReusableConnectionPool,
};

/// Connection pool wrapper to facilitate the use of pools in code under test and reusable pools in tests
pub enum PoolWrapper<B: Backend> {
    /// Connection pool used in code under test
    Pool(Pool<B::ConnectionManager>),
    /// Reusable connection pool used in tests
    ReusablePool(ReusableConnectionPool<'static, B>),
    /// Single-use connection pool used in tests
    SingleUsePool(SingleUseConnectionPool<B>),
}

impl<B: Backend> Deref for PoolWrapper<B> {
    type Target = Pool<B::ConnectionManager>;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Pool(pool) => pool,
            Self::ReusablePool(pool) => pool,
            Self::SingleUsePool(pool) => pool,
        }
    }
}

impl<B: Backend> From<ReusableConnectionPool<'static, B>> for PoolWrapper<B> {
    fn from(value: ReusableConnectionPool<'static, B>) -> Self {
        Self::ReusablePool(value)
    }
}

impl<B: Backend> From<SingleUseConnectionPool<B>> for PoolWrapper<B> {
    fn from(value: SingleUseConnectionPool<B>) -> Self {
        Self::SingleUsePool(value)
    }
}
