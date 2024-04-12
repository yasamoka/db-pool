use std::sync::Arc;

use super::{
    backend::{r#trait::Backend, Error},
    conn_pool::{ReusableConnectionPool as ReusableConnectionPoolInner, SingleUseConnectionPool},
    object_pool::{ObjectPool, Reusable},
};

/// Wrapper for a reusable connection pool wrapped in a reusable object wrapper
pub type ReusableConnectionPool<'a, B> = Reusable<'a, ReusableConnectionPoolInner<B>>;

/// Database pool
pub struct DatabasePool<B: Backend> {
    backend: Arc<B>,
    object_pool: ObjectPool<ReusableConnectionPoolInner<B>>,
}

impl<B: Backend> DatabasePool<B> {
    /// Pulls a reusable connection pool
    ///
    /// Privileges are granted only for ``SELECT``, ``INSERT``, ``UPDATE``, and ``DELETE`` operations.
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
    /// let conn_pool = db_pool.pull_immutable();
    /// ```
    #[must_use]
    pub fn pull_immutable(&self) -> Reusable<ReusableConnectionPoolInner<B>> {
        self.object_pool.pull()
    }

    /// Creates a single-use connection pool
    ///
    /// All privileges are granted.
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
    /// let conn_pool = db_pool.create_mutable();
    /// ```
    pub fn create_mutable(
        &self,
    ) -> Result<SingleUseConnectionPool<B>, Error<B::ConnectionError, B::QueryError>> {
        SingleUseConnectionPool::new(self.backend.clone())
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
        let object_pool = {
            let backend = backend.clone();
            ObjectPool::new(
                move || {
                    let backend = backend.clone();
                    ReusableConnectionPoolInner::new(backend)
                        .expect("connection pool creation must succeed")
                },
                |conn_pool| {
                    conn_pool
                        .clean()
                        .expect("connection pool cleaning must succeed");
                },
            )
        };
        Ok(DatabasePool {
            backend,
            object_pool,
        })
    }
}

impl<B> DatabasePoolBuilder for B where B: Backend + Sized {}
