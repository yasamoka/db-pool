use std::{
    borrow::Cow,
    fmt::Debug,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use async_trait::async_trait;
use uuid::Uuid;

use crate::{common::statement::mysql, util::get_db_name};

use super::super::error::Error as BackendError;

#[async_trait]
pub(super) trait MySQLBackend<'pool>: Send + Sync + 'static {
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

    async fn get_connection(&'pool self) -> Result<Self::PooledConnection, Self::PoolError>;

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

    fn get_host(&self) -> &str;

    async fn get_previous_database_names(
        &self,
        conn: &mut Self::Connection,
    ) -> Result<Vec<String>, Self::QueryError>;
    async fn create_entities(&self, db_name: &str) -> Result<(), Self::ConnectionError>;
    async fn create_connection_pool(&self, db_id: Uuid) -> Result<Self::Pool, Self::BuildError>;

    async fn get_table_names(
        &self,
        db_name: &str,
        conn: &mut Self::Connection,
    ) -> Result<Vec<String>, Self::QueryError>;

    fn get_drop_previous_databases(&self) -> bool;
}

pub(super) struct MySQLBackendWrapper<'backend, 'pool, B>
where
    B: MySQLBackend<'pool>,
{
    inner: &'backend B,
    _marker: &'pool PhantomData<()>,
}

impl<'backend, 'pool, B> MySQLBackendWrapper<'backend, 'pool, B>
where
    B: MySQLBackend<'pool>,
{
    pub(super) fn new(backend: &'backend B) -> Self {
        Self {
            inner: backend,
            _marker: &PhantomData,
        }
    }
}

impl<'backend, 'pool, B> Deref for MySQLBackendWrapper<'backend, 'pool, B>
where
    B: MySQLBackend<'pool>,
{
    type Target = B;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl<'backend, 'pool, B> MySQLBackendWrapper<'backend, 'pool, B>
where
    'backend: 'pool,
    B: MySQLBackend<'pool>,
{
    pub(super) async fn init(
        &'backend self,
    ) -> Result<(), BackendError<B::BuildError, B::PoolError, B::ConnectionError, B::QueryError>>
    {
        // Drop previous databases if needed
        if self.get_drop_previous_databases() {
            // Get privileged connection
            let conn = &mut self.get_connection().await.map_err(Into::into)?;

            // Get previous database names
            self.execute_stmt(mysql::USE_DEFAULT_DATABASE, conn)
                .await
                .map_err(Into::into)?;
            let mut db_names = self
                .get_previous_database_names(conn)
                .await
                .map_err(Into::into)?;

            // Drop databases
            let futures = db_names
                .drain(..)
                .map(|db_name| async move {
                    let conn = &mut self.get_connection().await.map_err(Into::into)?;
                    self.execute_stmt(mysql::drop_database(db_name.as_str()).as_str(), conn)
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
        db_id: uuid::Uuid,
    ) -> Result<B::Pool, BackendError<B::BuildError, B::PoolError, B::ConnectionError, B::QueryError>>
    {
        // Get database name based on UUID
        let db_name = get_db_name(db_id);
        let db_name = db_name.as_str();

        let host = self.get_host();

        // Get privileged connection
        let conn = &mut self.get_connection().await.map_err(Into::into)?;

        // Create database
        self.execute_stmt(mysql::create_database(db_name).as_str(), conn)
            .await
            .map_err(Into::into)?;

        // Create CRUD user
        self.execute_stmt(mysql::create_user(db_name, host).as_str(), conn)
            .await
            .map_err(Into::into)?;

        // Create entities
        self.execute_stmt(mysql::use_database(db_name).as_str(), conn)
            .await
            .map_err(Into::into)?;
        self.create_entities(db_name).await.map_err(Into::into)?;
        self.execute_stmt(mysql::USE_DEFAULT_DATABASE, conn)
            .await
            .map_err(Into::into)?;

        // Grant privileges to CRUD role
        self.execute_stmt(mysql::grant_privileges(db_name, host).as_str(), conn)
            .await
            .map_err(Into::into)?;

        // Create connection pool with CRUD role
        let pool = self
            .create_connection_pool(db_id)
            .await
            .map_err(Into::into)?;
        Ok(pool)
    }

    pub(super) async fn clean(
        &'backend self,
        db_id: uuid::Uuid,
    ) -> Result<(), BackendError<B::BuildError, B::PoolError, B::ConnectionError, B::QueryError>>
    {
        let db_name = get_db_name(db_id);
        let db_name = db_name.as_str();

        let conn = &mut self.get_connection().await.map_err(Into::into)?;

        let table_names = self
            .get_table_names(db_name, conn)
            .await
            .map_err(Into::into)?;
        let stmts = table_names
            .iter()
            .map(|table_name| mysql::truncate_table(table_name.as_str(), db_name).into());

        self.execute_stmt(mysql::TURN_OFF_FOREIGN_KEY_CHECKS, conn)
            .await
            .map_err(Into::into)?;
        self.batch_execute_stmt(stmts, conn)
            .await
            .map_err(Into::into)?;
        self.execute_stmt(mysql::TURN_ON_FOREIGN_KEY_CHECKS, conn)
            .await
            .map_err(Into::into)?;
        Ok(())
    }

    pub(super) async fn drop(
        &'backend self,
        db_id: uuid::Uuid,
    ) -> Result<(), BackendError<B::BuildError, B::PoolError, B::ConnectionError, B::QueryError>>
    {
        // Get database name based on UUID
        let db_name = get_db_name(db_id);
        let db_name = db_name.as_str();

        let host = self.get_host();

        // Get privileged connection
        let conn = &mut self.get_connection().await.map_err(Into::into)?;

        // Drop database
        self.execute_stmt(mysql::drop_database(db_name).as_str(), conn)
            .await
            .map_err(Into::into)?;

        // Drop CRUD role
        self.execute_stmt(mysql::drop_user(db_name, host).as_str(), conn)
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
        pooled_connection::AsyncDieselConnectionManager, AsyncMysqlConnection, RunQueryDsl,
    };
    use futures::{future::join_all, Future};
    use uuid::Uuid;

    use crate::{r#async::backend::r#trait::Backend, tests::MYSQL_DROP_LOCK};

    pub type Pool = Bb8Pool<AsyncDieselConnectionManager<AsyncMysqlConnection>>;

    pub const CREATE_ENTITIES_STMT: &str =
        "CREATE TABLE book(id INTEGER PRIMARY KEY AUTO_INCREMENT, title TEXT NOT NULL)";

    table! {
        schemata (schema_name) {
            schema_name -> Text
        }
    }

    #[allow(unused_variables)]
    trait MySQLDropLock<T>
    where
        Self: Future<Output = T> + Sized,
    {
        async fn lock_drop(self) -> T {
            let guard = MYSQL_DROP_LOCK.write().await;
            self.await
        }

        async fn lock_read(self) -> T {
            let guard = MYSQL_DROP_LOCK.read().await;
            self.await
        }
    }

    impl<T, F> MySQLDropLock<T> for F where F: Future<Output = T> + Sized {}

    async fn create_privileged_connection_pool() -> Pool {
        let manager = AsyncDieselConnectionManager::new("mysql://root:root@localhost:3306");
        Bb8Pool::builder().build(manager).await.unwrap()
    }

    async fn create_restricted_connection_pool(db_name: &str) -> Pool {
        let manager = AsyncDieselConnectionManager::new(format!(
            "mysql://{db_name}:{db_name}@localhost:3306/{db_name}"
        ));
        Bb8Pool::builder().build(manager).await.unwrap()
    }

    fn get_db_name(db_id: Uuid) -> String {
        let db_id = db_id.to_string().replace('-', "_");
        format!("db_pool_{db_id}")
    }

    async fn create_database(conn: &mut AsyncMysqlConnection) -> String {
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

    async fn use_database(db_name: &str, conn: &mut AsyncMysqlConnection) {
        sql_query(format!("USE {db_name}"))
            .execute(conn)
            .await
            .unwrap();
    }

    async fn use_information_schema(conn: &mut AsyncMysqlConnection) {
        use_database("information_schema", conn).await;
    }

    async fn count_databases(db_names: &Vec<String>, conn: &mut AsyncMysqlConnection) -> i64 {
        use_information_schema(conn).await;

        schemata::table
            .filter(schemata::schema_name.eq_any(db_names))
            .count()
            .get_result(conn)
            .await
            .unwrap()
    }

    async fn database_exists(db_name: &str, conn: &mut AsyncMysqlConnection) -> bool {
        use_information_schema(conn).await;

        select(exists(
            schemata::table.filter(schemata::schema_name.eq(db_name)),
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
            let conn_pool = create_privileged_connection_pool().await;
            let conn = &mut conn_pool.get().await.unwrap();

            for (backend, cleans) in [(default, true), (enabled, true), (disabled, false)] {
                let db_names = create_databases(NUM_DBS, &conn_pool).await;
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
        async {
            let db_id = Uuid::new_v4();
            let db_name = get_db_name(db_id);
            let db_name = db_name.as_str();

            // privileged operations
            {
                let conn_pool = create_privileged_connection_pool().await;
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
                let conn_pool = create_restricted_connection_pool(db_name).await;
                let conn = &mut conn_pool.get().await.unwrap();

                // // DDL statements must fail
                for stmt in [
                    "CREATE TABLE author(id INTEGER)",
                    "ALTER TABLE book RENAME TO new_book",
                    "ALTER TABLE book ADD description TEXT",
                    "ALTER TABLE book MODIFY title TEXT",
                    "ALTER TABLE book MODIFY title TEXT NOT NULL",
                    "ALTER TABLE book RENAME COLUMN title TO new_title",
                    "ALTER TABLE book CHANGE title new_title TEXT",
                    "ALTER TABLE book CHANGE title new_title TEXT NOT NULL",
                    "ALTER TABLE book DROP title",
                    "TRUNCATE TABLE book",
                    "DROP TABLE book",
                ] {
                    assert!(sql_query(stmt).execute(conn).await.is_err());
                }

                // DML statements must succeed
                for stmt in [
                    "SELECT * FROM book",
                    "INSERT INTO book (title) VALUES ('Title')",
                    "UPDATE book SET title = 'Title 2' WHERE id = 1",
                    "DELETE FROM book WHERE id = 1",
                ] {
                    assert!(sql_query(stmt).execute(conn).await.is_ok());
                }
            }
        }
        .lock_read()
        .await;
    }

    pub async fn test_cleans_database(backend: impl Backend) {
        const NUM_BOOKS: i64 = 3;

        async {
            let db_id = Uuid::new_v4();
            let db_name = get_db_name(db_id);
            let db_name = db_name.as_str();

            backend.init().await.unwrap();
            backend.create(db_id).await.unwrap();

            let conn_pool = create_privileged_connection_pool().await;
            let conn = &mut conn_pool.get().await.unwrap();

            use_database(db_name, conn).await;

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
        async {
            let db_id = Uuid::new_v4();
            let db_name = get_db_name(db_id);
            let db_name = db_name.as_str();

            let conn_pool = create_privileged_connection_pool().await;
            let conn = &mut conn_pool.get().await.unwrap();

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
