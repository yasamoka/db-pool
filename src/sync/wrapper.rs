use r2d2::{Error, Pool, PooledConnection};

use super::{backend::r#trait::Backend, conn_pool::ConnectionPool, object_pool::Reusable};

pub enum PoolWrapper<B>
where
    B: Backend,
{
    Pool(Pool<B::ConnectionManager>),
    ReusablePool(Reusable<'static, ConnectionPool<B>>),
}

impl<B> PoolWrapper<B>
where
    B: Backend,
{
    pub fn get(&self) -> Result<PooledConnection<B::ConnectionManager>, Error> {
        match self {
            Self::Pool(pool) => pool.get(),
            Self::ReusablePool(pool) => pool.get(),
        }
    }
}
