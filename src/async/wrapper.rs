use bb8::{ManageConnection, PooledConnection, RunError};

use super::{backend::AsyncBackend, conn_pool::AsyncConnectionPool, object_pool::AsyncReusable};

pub enum AsyncPoolWrapper<B>
where
    B: AsyncBackend,
{
    Pool(bb8::Pool<B::ConnectionManager>),
    ReusablePool(AsyncReusable<'static, AsyncConnectionPool<B>>),
}

impl<B> AsyncPoolWrapper<B>
where
    B: AsyncBackend,
{
    pub async fn get(
        &self,
    ) -> Result<
        PooledConnection<'_, B::ConnectionManager>,
        RunError<<B::ConnectionManager as ManageConnection>::Error>,
    > {
        match self {
            Self::Pool(pool) => pool.get(),
            Self::ReusablePool(pool) => pool.get(),
        }
        .await
    }
}
