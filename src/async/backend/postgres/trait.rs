use std::{borrow::Cow, fmt::Debug, marker::PhantomData, ops::DerefMut};

use async_trait::async_trait;
use uuid::Uuid;

use crate::{common::statement::postgres, util::get_db_name};

use super::super::error::Error as BackendError;

#[async_trait]
pub(super) trait PostgresBackend<'pool>: Send + Sync + 'static {
    type Connection;
    type PooledConnection: DerefMut<Target = Self::Connection>;
    type Pool;

    type BuildError: Into<
            BackendError<
                Self::BuildError,
                Self::PoolError,
                Self::ConnectionError,
                Self::QueryError,
            >,
        > + Debug;
    type PoolError: Into<
            BackendError<
                Self::BuildError,
                Self::PoolError,
                Self::ConnectionError,
                Self::QueryError,
            >,
        > + Debug;
    type ConnectionError: Into<
            BackendError<
                Self::BuildError,
                Self::PoolError,
                Self::ConnectionError,
                Self::QueryError,
            >,
        > + Debug;
    type QueryError: Into<
            BackendError<
                Self::BuildError,
                Self::PoolError,
                Self::ConnectionError,
                Self::QueryError,
            >,
        > + Debug;

    async fn execute_stmt(
        &self,
        query: &str,
        conn: &mut Self::Connection,
    ) -> Result<(), Self::QueryError>;
    async fn batch_execute_stmt<'a>(
        &self,
        query: impl IntoIterator<Item = Cow<'a, str>> + Send,
        conn: &mut Self::Connection,
    ) -> Result<(), Self::QueryError>;

    async fn get_default_connection(&'pool self)
        -> Result<Self::PooledConnection, Self::PoolError>;
    async fn establish_database_connection(
        &self,
        db_id: Uuid,
    ) -> Result<Self::Connection, Self::ConnectionError>;
    fn put_database_connection(&self, db_id: Uuid, conn: Self::Connection);
    fn get_database_connection(&self, db_id: Uuid) -> Self::Connection;

    async fn get_previous_database_names(
        &self,
        conn: &mut Self::Connection,
    ) -> Result<Vec<String>, Self::QueryError>;
    async fn create_entities(&self, conn: Self::Connection) -> Self::Connection;
    async fn create_connection_pool(&self, db_id: Uuid) -> Result<Self::Pool, Self::BuildError>;

    async fn get_table_names(
        &self,
        privileged_conn: &mut Self::Connection,
    ) -> Result<Vec<String>, Self::QueryError>;

    fn get_drop_previous_databases(&self) -> bool;
}

pub(super) struct PostgresBackendWrapper<'backend, 'pool, B>
where
    B: PostgresBackend<'pool>,
{
    inner: &'backend B,
    _marker: &'pool PhantomData<()>,
}

impl<'backend, 'pool, B> PostgresBackendWrapper<'backend, 'pool, B>
where
    B: PostgresBackend<'pool>,
{
    pub(super) fn new(backend: &'backend B) -> Self {
        Self {
            inner: backend,
            _marker: &PhantomData,
        }
    }
}

impl<'backend, 'pool, B> PostgresBackendWrapper<'backend, 'pool, B>
where
    'backend: 'pool,
    B: PostgresBackend<'pool>,
{
    pub(super) async fn init(
        &'backend self,
    ) -> Result<(), BackendError<B::BuildError, B::PoolError, B::ConnectionError, B::QueryError>>
    {
        // Drop previous databases if needed
        if self.inner.get_drop_previous_databases() {
            // Get connection to default database as privileged user
            let conn = &mut self
                .inner
                .get_default_connection()
                .await
                .map_err(Into::into)?;

            // Get previous database names
            let db_names = self
                .inner
                .get_previous_database_names(conn)
                .await
                .map_err(Into::into)?;

            // Drop databases
            let futures = db_names
                .iter()
                .map(|db_name| async move {
                    let conn = &mut self
                        .inner
                        .get_default_connection()
                        .await
                        .map_err(Into::into)?;
                    self.inner
                        .execute_stmt(postgres::drop_database(db_name.as_str()).as_str(), conn)
                        .await
                        .map_err(Into::into)?;
                    Ok::<
                        _,
                        BackendError<
                            B::BuildError,
                            B::PoolError,
                            B::ConnectionError,
                            B::QueryError,
                        >,
                    >(())
                })
                .collect::<Vec<_>>();
            futures::future::try_join_all(futures).await?;
        }

        Ok(())
    }

    pub(super) async fn create(
        &'backend self,
        db_id: Uuid,
    ) -> Result<B::Pool, BackendError<B::BuildError, B::PoolError, B::ConnectionError, B::QueryError>>
    {
        // Get database name based on UUID
        let db_name = get_db_name(db_id);
        let db_name = db_name.as_str();

        {
            // Get connection to default database as privileged user
            let conn = &mut self
                .inner
                .get_default_connection()
                .await
                .map_err(Into::into)?;

            // Create database
            self.inner
                .execute_stmt(postgres::create_database(db_name).as_str(), conn)
                .await
                .map_err(Into::into)?;

            // Create CRUD role
            self.inner
                .execute_stmt(postgres::create_role(db_name).as_str(), conn)
                .await
                .map_err(Into::into)?;
        }

        {
            // Connect to database as privileged user
            let conn = self
                .inner
                .establish_database_connection(db_id)
                .await
                .map_err(Into::into)?;

            // Create entities
            let mut conn = self.inner.create_entities(conn).await;

            // Grant privileges to CRUD role
            self.inner
                .execute_stmt(
                    postgres::grant_table_privileges(db_name).as_str(),
                    &mut conn,
                )
                .await
                .map_err(Into::into)?;
            self.inner
                .execute_stmt(
                    postgres::grant_sequence_privileges(db_name).as_str(),
                    &mut conn,
                )
                .await
                .map_err(Into::into)?;

            // Store database connection for reuse when cleaning
            self.inner.put_database_connection(db_id, conn);
        }

        // Create connection pool with CRUD role
        let pool = self
            .inner
            .create_connection_pool(db_id)
            .await
            .map_err(Into::into)?;
        Ok(pool)
    }

    pub(super) async fn clean(
        &'backend self,
        db_id: Uuid,
    ) -> Result<(), BackendError<B::BuildError, B::PoolError, B::ConnectionError, B::QueryError>>
    {
        let mut conn = self.inner.get_database_connection(db_id);
        let table_names = self
            .inner
            .get_table_names(&mut conn)
            .await
            .map_err(Into::into)?;
        let stmts = table_names
            .iter()
            .map(|table_name| postgres::truncate_table(table_name.as_str()).into());
        self.inner
            .batch_execute_stmt(stmts, &mut conn)
            .await
            .map_err(Into::into)?;
        self.inner.put_database_connection(db_id, conn);
        Ok(())
    }

    pub(super) async fn drop(
        &'backend self,
        db_id: Uuid,
    ) -> Result<(), BackendError<B::BuildError, B::PoolError, B::ConnectionError, B::QueryError>>
    {
        // Drop privileged connection to database
        {
            self.inner.get_database_connection(db_id);
        }

        // Get database name based on UUID
        let db_name = get_db_name(db_id);
        let db_name = db_name.as_str();

        // Get connection to default database as privileged user
        let conn = &mut self
            .inner
            .get_default_connection()
            .await
            .map_err(Into::into)?;

        // Drop database
        self.inner
            .execute_stmt(postgres::drop_database(db_name).as_str(), conn)
            .await
            .map_err(Into::into)?;

        // Drop CRUD role
        self.inner
            .execute_stmt(postgres::drop_role(db_name).as_str(), conn)
            .await
            .map_err(Into::into)?;

        Ok(())
    }
}

#[cfg(test)]
pub(super) mod tests {
    #![allow(clippy::unwrap_used)]

    use bb8::Pool as Bb8Pool;
    use diesel::{dsl::exists, insert_into, prelude::*, select, sql_query, table};
    use diesel_async::{
        pooled_connection::AsyncDieselConnectionManager, AsyncPgConnection, RunQueryDsl,
    };
    use futures::{future::join_all, Future};
    use tokio::sync::OnceCell;
    use uuid::Uuid;

    use crate::{
        common::statement::postgres::tests::{DDL_STATEMENTS, DML_STATEMENTS},
        r#async::backend::r#trait::Backend,
        tests::PG_DROP_LOCK,
        util::get_db_name,
    };

    pub type Pool = Bb8Pool<AsyncDieselConnectionManager<AsyncPgConnection>>;

    table! {
        pg_database (oid) {
            oid -> Int4,
            datname -> Text
        }
    }

    #[allow(unused_variables)]
    trait DropLock<T>
    where
        Self: Future<Output = T> + Sized,
    {
        async fn lock_drop(self) -> T {
            let guard = PG_DROP_LOCK.write().await;
            self.await
        }

        async fn lock_read(self) -> T {
            let guard = PG_DROP_LOCK.read().await;
            self.await
        }
    }

    impl<T, F> DropLock<T> for F where F: Future<Output = T> + Sized {}

    async fn get_privileged_connection_pool() -> &'static Pool {
        static POOL: OnceCell<Pool> = OnceCell::const_new();
        POOL.get_or_init(|| async {
            let manager =
                AsyncDieselConnectionManager::new("postgres://postgres:postgres@localhost:5432");
            Bb8Pool::builder().build(manager).await.unwrap()
        })
        .await
    }

    async fn create_restricted_connection_pool(db_name: &str) -> Pool {
        let manager = AsyncDieselConnectionManager::new(format!(
            "postgres://{db_name}:{db_name}@localhost:5432/{db_name}"
        ));
        Bb8Pool::builder().build(manager).await.unwrap()
    }

    async fn create_database(conn: &mut AsyncPgConnection) -> String {
        let db_id = Uuid::new_v4();
        let db_name = get_db_name(db_id);
        sql_query(format!("CREATE DATABASE {db_name}"))
            .execute(conn)
            .await
            .unwrap();
        db_name
    }

    async fn create_databases(count: i64, pool: &Pool) -> Vec<String> {
        let futures = (0..count)
            .map(|_| async {
                let conn = &mut pool.get().await.unwrap();
                create_database(conn).await
            })
            .collect::<Vec<_>>();
        join_all(futures).await
    }

    async fn count_databases(db_names: &Vec<String>, conn: &mut AsyncPgConnection) -> i64 {
        pg_database::table
            .filter(pg_database::datname.eq_any(db_names))
            .count()
            .get_result(conn)
            .await
            .unwrap()
    }

    async fn database_exists(db_name: &str, conn: &mut AsyncPgConnection) -> bool {
        select(exists(
            pg_database::table.filter(pg_database::datname.eq(db_name)),
        ))
        .get_result(conn)
        .await
        .unwrap()
    }

    pub async fn test_drops_previous_databases<B>(default: B, enabled: B, disabled: B)
    where
        B: Backend,
    {
        const NUM_DBS: i64 = 3;

        async {
            let conn_pool = get_privileged_connection_pool().await;
            let conn = &mut conn_pool.get().await.unwrap();

            for (backend, cleans) in [(default, true), (enabled, true), (disabled, false)] {
                let db_names = create_databases(NUM_DBS, conn_pool).await;
                assert_eq!(count_databases(&db_names, conn).await, NUM_DBS);
                backend.init().await.unwrap();
                assert_eq!(
                    count_databases(&db_names, conn).await,
                    if cleans { 0 } else { NUM_DBS }
                );
            }
        }
        .lock_drop()
        .await;
    }

    pub async fn test_creates_database_with_restricted_privileges(backend: impl Backend) {
        let db_id = Uuid::new_v4();
        let db_name = get_db_name(db_id);
        let db_name = db_name.as_str();

        async {
            // privileged operations
            {
                let conn_pool = get_privileged_connection_pool().await;
                let conn = &mut conn_pool.get().await.unwrap();

                // database must not exist
                assert!(!database_exists(db_name, conn).await);

                // database must exist after creating through backend
                backend.init().await.unwrap();
                backend.create(db_id).await.unwrap();
                assert!(database_exists(db_name, conn).await);
            }

            // restricted operations
            {
                let conn_pool = &mut create_restricted_connection_pool(db_name).await;
                let conn = &mut conn_pool.get().await.unwrap();

                // DDL statements must fail
                for stmt in DDL_STATEMENTS {
                    assert!(sql_query(stmt).execute(conn).await.is_err());
                }

                // DML statements must succeed
                for stmt in DML_STATEMENTS {
                    assert!(sql_query(stmt).execute(conn).await.is_ok());
                }
            }
        }
        .lock_read()
        .await;
    }

    pub async fn test_cleans_database(backend: impl Backend) {
        const NUM_BOOKS: i64 = 3;

        let db_id = Uuid::new_v4();
        let db_name = get_db_name(db_id);
        let db_name = db_name.as_str();

        async {
            backend.init().await.unwrap();
            backend.create(db_id).await.unwrap();

            let conn_pool = &mut create_restricted_connection_pool(db_name).await;
            let conn = &mut conn_pool.get().await.unwrap();

            table! {
                book (id) {
                    id -> Int4,
                    title -> Text
                }
            }

            #[derive(Insertable)]
            #[diesel(table_name = book)]
            struct NewBook {
                title: String,
            }

            let new_books = (0..NUM_BOOKS)
                .map(|i| NewBook {
                    title: format!("Title {}", i + 1),
                })
                .collect::<Vec<_>>();
            insert_into(book::table)
                .values(&new_books)
                .execute(conn)
                .await
                .unwrap();

            // there must be books
            assert_eq!(
                book::table.count().get_result::<i64>(conn).await.unwrap(),
                NUM_BOOKS
            );

            backend.clean(db_id).await.unwrap();

            // there must be no books
            assert_eq!(
                book::table.count().get_result::<i64>(conn).await.unwrap(),
                0
            );
        }
        .lock_read()
        .await;
    }

    pub async fn test_drops_database(backend: impl Backend) {
        let db_id = Uuid::new_v4();
        let db_name = get_db_name(db_id);
        let db_name = db_name.as_str();

        let conn_pool = get_privileged_connection_pool().await;
        let conn = &mut conn_pool.get().await.unwrap();

        async {
            // database must exist
            backend.init().await.unwrap();
            backend.create(db_id).await.unwrap();
            assert!(database_exists(db_name, conn).await);

            // database must not exist
            backend.drop(db_id).await.unwrap();
            assert!(!database_exists(db_name, conn).await);
        }
        .lock_read()
        .await;
    }
}
