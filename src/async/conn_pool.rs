use std::{ops::Deref, sync::Arc};

use uuid::Uuid;

use super::backend::{r#trait::Backend, Error as BackendError};

struct ConnectionPool<B: Backend> {
    backend: Arc<B>,
    db_id: Uuid,
    conn_pool: Option<B::Pool>,
    is_restricted: bool
}

impl<B: Backend> Deref for ConnectionPool<B> {
    type Target = B::Pool;

    fn deref(&self) -> &Self::Target {
        self.conn_pool
            .as_ref()
            .expect("conn_pool must always contain a [Some] value")
    }
}

impl<B: Backend> Drop for ConnectionPool<B> {
    fn drop(&mut self) {
        self.conn_pool = None;
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                (*self.backend)
                    .drop(self.db_id, self.is_restricted)
                    .await
                    .ok();
            });
        });
    }
}

/// Reusable connection pool wrapper
pub struct ReusableConnectionPool<B: Backend>(ConnectionPool<B>);

impl<B: Backend> ReusableConnectionPool<B> {
    pub(crate) async fn new(
        backend: Arc<B>,
    ) -> Result<Self, BackendError<B::BuildError, B::PoolError, B::ConnectionError, B::QueryError>>
    {
        let db_id = Uuid::new_v4();
        let conn_pool = backend.create(db_id, true).await?;

        Ok(Self(ConnectionPool {
            backend,
            db_id,
            conn_pool: Some(conn_pool),
            is_restricted: true
        }))
    }

    pub(crate) async fn clean(
        &mut self,
    ) -> Result<(), BackendError<B::BuildError, B::PoolError, B::ConnectionError, B::QueryError>>
    {
        self.0.backend.clean(self.0.db_id).await
    }
}

impl<B: Backend> Deref for ReusableConnectionPool<B> {
    type Target = B::Pool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Single-use connection pool wrapper
pub struct SingleUseConnectionPool<B: Backend>(ConnectionPool<B>);

impl<B: Backend> SingleUseConnectionPool<B> {
    pub(crate) async fn new(
        backend: Arc<B>,
    ) -> Result<Self, BackendError<B::BuildError, B::PoolError, B::ConnectionError, B::QueryError>>
    {
        let db_id = Uuid::new_v4();
        let conn_pool = backend.create(db_id, false).await?;

        Ok(Self(ConnectionPool {
                backend,
                db_id,
                conn_pool: Some(conn_pool),
                is_restricted: false
            },
        ))
    }
}

impl<B: Backend> Deref for SingleUseConnectionPool<B> {
    type Target = B::Pool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
