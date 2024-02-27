use std::{ops::Deref, sync::Arc};

use bb8::Pool;
use uuid::Uuid;

use crate::util::get_db_name;

use super::backend::AsyncBackend;

pub struct ReusableAsyncConnectionPool<B>
where
    B: AsyncBackend,
{
    backend: Arc<B>,
    db_id: Uuid,
    conn_pool: Option<Pool<B::ConnectionManager>>,
}

impl<B> ReusableAsyncConnectionPool<B>
where
    B: AsyncBackend,
{
    pub async fn new(backend: Arc<B>) -> Self {
        let db_id = Uuid::new_v4();
        let conn_pool = backend.create(db_id).await;

        Self {
            backend,
            db_id,
            conn_pool: Some(conn_pool),
        }
    }

    pub fn db_name(&self) -> String {
        get_db_name(self.db_id)
    }

    pub async fn clean(&mut self) {
        self.backend.clean(self.db_id).await;
    }
}

impl<B> Deref for ReusableAsyncConnectionPool<B>
where
    B: AsyncBackend,
{
    type Target = Pool<B::ConnectionManager>;

    fn deref(&self) -> &Self::Target {
        self.conn_pool.as_ref().unwrap()
    }
}

impl<B> Drop for ReusableAsyncConnectionPool<B>
where
    B: AsyncBackend,
{
    fn drop(&mut self) {
        self.conn_pool = None;
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                (*self.backend).drop(self.db_id).await;
            });
        });
    }
}
