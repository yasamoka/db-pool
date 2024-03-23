use std::sync::Arc;

use super::{
    backend::{r#trait::Backend, Error},
    conn_pool::ConnectionPool,
    object_pool::{ObjectPool, Reusable},
};

/// Database pool
pub struct DatabasePool<B>(Arc<ObjectPool<ConnectionPool<B>>>)
where
    B: Backend;

impl<B> DatabasePool<B>
where
    B: Backend,
{
    /// Pulls a reusable connection pool
    /// # Example
    /// ```
    /// use db_pool::{
    ///     sync::{DatabasePoolBuilderTrait, DieselPostgresBackend},
    ///     PrivilegedPostgresConfig,
    /// };
    /// use diesel::{sql_query, RunQueryDsl};
    /// use dotenvy::dotenv;
    /// use r2d2::Pool;
    ///
    /// dotenv().ok();
    ///
    /// let config = PrivilegedPostgresConfig::from_env().unwrap();
    ///
    /// let backend = DieselPostgresBackend::new(
    ///     config,
    ///     || Pool::builder().max_size(10),
    ///     || Pool::builder().max_size(2),
    ///     move |conn| {
    ///         sql_query("CREATE TABLE book(id SERIAL PRIMARY KEY, title TEXT NOT NULL)")
    ///             .execute(conn)
    ///             .unwrap();
    ///     },
    /// )
    /// .unwrap();
    ///
    /// let db_pool = backend.create_database_pool().unwrap();
    /// let conn_pool = db_pool.pull();
    /// ```
    #[must_use]
    pub fn pull(&self) -> Reusable<ConnectionPool<B>> {
        self.0.pull()
    }
}

impl<B> Clone for DatabasePool<B>
where
    B: Backend,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

/// Database pool builder trait implemented for all sync backends
pub trait DatabasePoolBuilder: Backend {
    /// Creates a database pool
    /// # Example
    /// ```
    /// use db_pool::{
    ///     sync::{DatabasePoolBuilderTrait, DieselPostgresBackend},
    ///     PrivilegedPostgresConfig,
    /// };
    /// use diesel::{sql_query, RunQueryDsl};
    /// use dotenvy::dotenv;
    /// use r2d2::Pool;
    ///
    /// dotenv().ok();
    ///
    /// let config = PrivilegedPostgresConfig::from_env().unwrap();
    ///
    /// let backend = DieselPostgresBackend::new(
    ///     config,
    ///     || Pool::builder().max_size(10),
    ///     || Pool::builder().max_size(2),
    ///     move |conn| {
    ///         sql_query("CREATE TABLE book(id SERIAL PRIMARY KEY, title TEXT NOT NULL)")
    ///             .execute(conn)
    ///             .unwrap();
    ///     },
    /// )
    /// .unwrap();
    ///
    /// let db_pool = backend.create_database_pool().unwrap();
    /// ```
    fn create_database_pool(
        self,
    ) -> Result<DatabasePool<Self>, Error<Self::ConnectionError, Self::QueryError>> {
        self.init()?;
        let backend = Arc::new(self);
        let object_pool = Arc::new(ObjectPool::new(
            move || {
                let backend = backend.clone();
                ConnectionPool::new(backend).expect("connection pool creation must succeed")
            },
            |conn_pool| {
                conn_pool
                    .clean()
                    .expect("connection pool cleaning must succeed");
            },
        ));
        Ok(DatabasePool(object_pool))
    }
}

impl<B> DatabasePoolBuilder for B where B: Backend + Sized {}
