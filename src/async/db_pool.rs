use std::sync::Arc;

use async_trait::async_trait;

use super::{
    backend::{r#trait::Backend, Error},
    conn_pool::ConnectionPool,
    object_pool::{ObjectPool, Reusable},
};

/// Database pool
pub struct DatabasePool<B: Backend>(Arc<ObjectPool<ConnectionPool<B>>>);

impl<B: Backend> DatabasePool<B> {
    /// Pulls a reusable connection pool
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
    ///     let conn_pool = db_pool.pull();
    /// }
    ///
    /// tokio_test::block_on(f());
    /// ```
    #[must_use]
    pub async fn pull(&self) -> Reusable<ConnectionPool<B>> {
        self.0.pull().await
    }
}

impl<B: Backend> Clone for DatabasePool<B> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
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
        let object_pool = Arc::new(ObjectPool::new(
            move || {
                let backend = backend.clone();
                Box::pin(async {
                    ConnectionPool::new(backend)
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
        ));
        Ok(DatabasePool(object_pool))
    }
}

impl<AB: Backend> DatabasePoolBuilder for AB {}
