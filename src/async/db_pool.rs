use std::sync::Arc;

use async_trait::async_trait;

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
    /// use bb8::Pool;
    /// use db_pool::{
    ///     r#async::{DatabasePoolBuilderTrait, DieselAsyncPostgresBackend, DieselBb8},
    ///     PrivilegedPostgresConfig,
    /// };
    /// use diesel::sql_query;
    /// use diesel_async::RunQueryDsl;
    /// use dotenvy::dotenv;
    ///
    /// async fn f() {
    ///     dotenv().ok();
    ///
    ///     let config = PrivilegedPostgresConfig::from_env().unwrap();
    ///
    ///     let backend = DieselAsyncPostgresBackend::<DieselBb8>::new(
    ///         config,
    ///         || Pool::builder().max_size(10),
    ///         || Pool::builder().max_size(2),
    ///         move |mut conn| {
    ///             Box::pin(async {
    ///                 sql_query("CREATE TABLE book(id SERIAL PRIMARY KEY, title TEXT NOT NULL)")
    ///                     .execute(&mut conn)
    ///                     .await
    ///                     .unwrap();
    ///                 conn
    ///             })
    ///         },
    ///     )
    ///     .await
    ///     .unwrap();
    ///
    ///     let db_pool = backend.create_database_pool().await.unwrap();
    ///     let conn_pool = db_pool.pull_immutable();
    /// }
    ///
    /// tokio_test::block_on(f());
    /// ```
    #[must_use]
    pub async fn pull_immutable(&self) -> ReusableConnectionPool<B> {
        self.object_pool.pull().await
    }

    /// Creates a single-use connection pool
    ///
    /// All privileges are granted.
    /// # Example
    /// ```
    /// use bb8::Pool;
    /// use db_pool::{
    ///     r#async::{DatabasePoolBuilderTrait, DieselAsyncPostgresBackend, DieselBb8},
    ///     PrivilegedPostgresConfig,
    /// };
    /// use diesel::sql_query;
    /// use diesel_async::RunQueryDsl;
    /// use dotenvy::dotenv;
    ///
    /// async fn f() {
    ///     dotenv().ok();
    ///
    ///     let config = PrivilegedPostgresConfig::from_env().unwrap();
    ///
    ///     let backend = DieselAsyncPostgresBackend::<DieselBb8>::new(
    ///         config,
    ///         || Pool::builder().max_size(10),
    ///         || Pool::builder().max_size(2),
    ///         move |mut conn| {
    ///             Box::pin(async {
    ///                 sql_query("CREATE TABLE book(id SERIAL PRIMARY KEY, title TEXT NOT NULL)")
    ///                     .execute(&mut conn)
    ///                     .await
    ///                     .unwrap();
    ///                 conn
    ///             })
    ///         },
    ///     )
    ///     .await
    ///     .unwrap();
    ///
    ///     let db_pool = backend.create_database_pool().await.unwrap();
    ///     let conn_pool = db_pool.create_mutable();
    /// }
    ///
    /// tokio_test::block_on(f());
    /// ```
    pub async fn create_mutable(
        &self,
    ) -> Result<
        SingleUseConnectionPool<B>,
        Error<B::BuildError, B::PoolError, B::ConnectionError, B::QueryError>,
    > {
        SingleUseConnectionPool::new(self.backend.clone()).await
    }
}

/// Database pool builder trait implemented for all async backends
#[async_trait]
pub trait DatabasePoolBuilder: Backend {
    /// Creates a database pool
    /// # Example
    /// ```
    /// use bb8::Pool;
    /// use db_pool::{
    ///     r#async::{DatabasePoolBuilderTrait, DieselAsyncPostgresBackend, DieselBb8},
    ///     PrivilegedPostgresConfig,
    /// };
    /// use diesel::sql_query;
    /// use diesel_async::RunQueryDsl;
    /// use dotenvy::dotenv;
    ///
    /// async fn f() {
    ///     dotenv().ok();
    ///
    ///     let config = PrivilegedPostgresConfig::from_env().unwrap();
    ///
    ///     let backend = DieselAsyncPostgresBackend::<DieselBb8>::new(
    ///         config,
    ///         || Pool::builder().max_size(10),
    ///         || Pool::builder().max_size(2),
    ///         move |mut conn| {
    ///             Box::pin(async {
    ///                 sql_query("CREATE TABLE book(id SERIAL PRIMARY KEY, title TEXT NOT NULL)")
    ///                     .execute(&mut conn)
    ///                     .await
    ///                     .unwrap();
    ///                 conn
    ///             })
    ///         },
    ///     )
    ///     .await
    ///     .unwrap();
    ///
    ///     let db_pool = backend.create_database_pool().await.unwrap();
    /// }
    ///
    /// tokio_test::block_on(f());
    /// ```
    async fn create_database_pool(
        self,
    ) -> Result<
        DatabasePool<Self>,
        Error<Self::BuildError, Self::PoolError, Self::ConnectionError, Self::QueryError>,
    > {
        self.init().await?;
        let backend = Arc::new(self);
        let object_pool = {
            let backend = backend.clone();
            ObjectPool::new(
                move || {
                    let backend = backend.clone();
                    Box::pin(async {
                        ReusableConnectionPoolInner::new(backend)
                            .await
                            .expect("connection pool creation must succeed")
                    })
                },
                |mut conn_pool| {
                    Box::pin(async {
                        conn_pool
                            .clean()
                            .await
                            .expect("connection pool cleaning must succeed");
                        conn_pool
                    })
                },
            )
        };
        Ok(DatabasePool {
            backend,
            object_pool,
        })
    }
}

impl<AB: Backend> DatabasePoolBuilder for AB {}
