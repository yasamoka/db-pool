use std::ops::Deref;

use super::{backend::r#trait::Backend, conn_pool::ConnectionPool, object_pool::Reusable};

pub enum PoolWrapper<B>
where
    B: Backend,
{
    Pool(B::Pool),
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
