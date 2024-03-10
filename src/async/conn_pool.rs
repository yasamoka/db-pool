use std::{ops::Deref, sync::Arc};

use bb8::{ManageConnection, Pool};
use uuid::Uuid;

use crate::util::get_db_name;

use super::backend::{r#trait::Backend, Error as BackendError};

pub struct ConnectionPool<B>
where
    B: Backend,
{
    backend: Arc<B>,
    db_id: Uuid,
    conn_pool: Option<Pool<B::ConnectionManager>>,
}

impl<B> ConnectionPool<B>
where
    B: Backend,
{
    pub async fn new(
        backend: Arc<B>,
    ) -> Result<
        Self,
        BackendError<
            <B::ConnectionManager as ManageConnection>::Error,
            B::ConnectionError,
            B::QueryError,
        >,
    > {
        let db_id = Uuid::new_v4();
        let conn_pool = backend.create(db_id).await?;

        Ok(Self {
            backend,
            db_id,
            conn_pool: Some(conn_pool),
        })
    }

    #[must_use]
    pub fn db_name(&self) -> String {
        get_db_name(self.db_id)
    }

    pub async fn clean(
        &mut self,
    ) -> Result<
        (),
        BackendError<
            <B::ConnectionManager as ManageConnection>::Error,
            B::ConnectionError,
            B::QueryError,
        >,
    > {
        self.backend.clean(self.db_id).await
    }
}

impl<B> Deref for ConnectionPool<B>
where
    B: Backend,
{
    type Target = Pool<B::ConnectionManager>;

    fn deref(&self) -> &Self::Target {
        self.conn_pool
            .as_ref()
            .expect("conn_pool must always contain a [Some] value")
    }
}

impl<B> Drop for ConnectionPool<B>
where
    B: Backend,
{
    fn drop(&mut self) {
        self.conn_pool = None;
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                (*self.backend).drop(self.db_id).await.ok();
            });
        });
    }
}
