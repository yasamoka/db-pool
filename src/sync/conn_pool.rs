use std::{ops::Deref, sync::Arc};

use r2d2::Pool;
use uuid::Uuid;

use crate::util::get_db_name;

use super::backend::Backend;

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
    pub fn new(backend: Arc<B>) -> Self {
        let db_id = Uuid::new_v4();
        let conn_pool = backend.create(db_id);

        Self {
            backend,
            db_id,
            conn_pool: Some(conn_pool),
        }
    }

    #[must_use]
    pub fn db_name(&self) -> String {
        get_db_name(self.db_id)
    }

    pub fn clean(&mut self) {
        self.backend.clean(self.db_id);
    }
}

impl<B> Deref for ConnectionPool<B>
where
    B: Backend,
{
    type Target = Pool<B::ConnectionManager>;

    fn deref(&self) -> &Self::Target {
        self.conn_pool.as_ref().unwrap()
    }
}

impl<B> Drop for ConnectionPool<B>
where
    B: Backend,
{
    fn drop(&mut self) {
        self.conn_pool = None;
        (*self.backend).drop(self.db_id);
    }
}
