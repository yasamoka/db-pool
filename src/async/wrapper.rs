use std::ops::Deref;

use super::{backend::r#trait::Backend, conn_pool::ConnectionPool, object_pool::Reusable};

/// Connection pool wrapper to facilitate the use of pools in code under test and reusable pools in tests
pub enum PoolWrapper<B>
where
    B: Backend,
{
    /// Connection pool used in code under test
    Pool(B::Pool),
    /// Reusable connection pool used in tests
    ReusablePool(Reusable<'static, ConnectionPool<B>>),
}

impl<B> Deref for PoolWrapper<B>
where
    B: Backend,
{
    type Target = B::Pool;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Pool(pool) => pool,
            Self::ReusablePool(pool) => pool,
        }
    }
}
