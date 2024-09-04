use std::{borrow::Cow, pin::Pin};

use async_trait::async_trait;
use futures::Future;
use sqlx::{
    mysql::{MySqlConnectOptions, MySqlPoolOptions},
    pool::PoolConnection,
    Connection, Executor, MySql, MySqlConnection, MySqlPool, Row,
};
use uuid::Uuid;

use crate::{common::statement::mysql, util::get_db_name};

use super::{
    super::{
        common::error::sqlx::{BuildError, ConnectionError, PoolError, QueryError},
        error::Error as BackendError,
        r#trait::Backend,
    },
    r#trait::{MySQLBackend, MySQLBackendWrapper},
};

type CreateEntities = dyn Fn(MySqlConnection) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
    + Send
    + Sync
    + 'static;

/// [`sqlx MySQL`](https://docs.rs/sqlx/0.8.2/sqlx/struct.MySql.html) backend
pub struct SqlxMySQLBackend {
    privileged_opts: MySqlConnectOptions,
    default_pool: MySqlPool,
    create_restricted_pool: Box<dyn Fn() -> MySqlPoolOptions + Send + Sync + 'static>,
    create_entities: Box<CreateEntities>,
    drop_previous_databases_flag: bool,
}

impl SqlxMySQLBackend {
    /// Creates a new [`sqlx MySQL`](https://docs.rs/sqlx/0.8.2/sqlx/struct.MySql.html) backend
    /// # Example
    /// ```
    /// use db_pool::{r#async::SqlxMySQLBackend, PrivilegedMySQLConfig};
    /// use dotenvy::dotenv;
    /// use sqlx::{mysql::MySqlPoolOptions, Executor};
    ///
    /// async fn f() {
    ///     dotenv().ok();
    ///
    ///     let config = PrivilegedMySQLConfig::from_env().unwrap();
    ///
    ///     let backend = SqlxMySQLBackend::new(
    ///         config.into(),
    ///         || MySqlPoolOptions::new().max_connections(10),
    ///         || MySqlPoolOptions::new().max_connections(2),
    ///         move |mut conn| {
    ///             Box::pin(async move {
    ///                 conn.execute("CREATE TABLE book(id INTEGER PRIMARY KEY AUTO_INCREMENT, title TEXT NOT NULL)")
    ///                      .await
    ///                      .unwrap();
    ///             })
    ///         },
    ///     );
    /// }
    ///
    /// tokio_test::block_on(f());
    /// ```
    pub fn new(
        privileged_options: MySqlConnectOptions,
        create_privileged_pool: impl Fn() -> MySqlPoolOptions,
        create_restricted_pool: impl Fn() -> MySqlPoolOptions + Send + Sync + 'static,
        create_entities: impl Fn(MySqlConnection) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        let pool_opts = create_privileged_pool();
        let default_pool = pool_opts.connect_lazy_with(privileged_options.clone());

        Self {
            privileged_opts: privileged_options,
            default_pool,
            create_restricted_pool: Box::new(create_restricted_pool),
            create_entities: Box::new(create_entities),
            drop_previous_databases_flag: true,
        }
    }

    /// Drop databases created in previous runs upon initialization
    #[must_use]
    pub fn drop_previous_databases(self, value: bool) -> Self {
        Self {
            drop_previous_databases_flag: value,
            ..self
        }
    }
}

#[async_trait]
impl<'pool> MySQLBackend<'pool> for SqlxMySQLBackend {
    type Connection = MySqlConnection;
    type PooledConnection = PoolConnection<MySql>;
    type Pool = MySqlPool;

    type BuildError = BuildError;
    type PoolError = PoolError;
    type ConnectionError = ConnectionError;
    type QueryError = QueryError;

    async fn get_connection(&'pool self) -> Result<PoolConnection<MySql>, PoolError> {
        self.default_pool.acquire().await.map_err(Into::into)
    }

    async fn execute_query(
        &self,
        query: &str,
        conn: &mut MySqlConnection,
    ) -> Result<(), QueryError> {
        conn.execute(query).await?;
        Ok(())
    }

    async fn batch_execute_query<'a>(
        &self,
        query: impl IntoIterator<Item = Cow<'a, str>> + Send,
        conn: &mut MySqlConnection,
    ) -> Result<(), QueryError> {
        let chunks = query.into_iter().collect::<Vec<_>>();
        if chunks.is_empty() {
            Ok(())
        } else {
            let query = chunks.join(";");
            self.execute_query(query.as_str(), conn).await
        }
    }

    fn get_host(&self) -> &str {
        self.privileged_opts.get_host()
    }

    async fn get_previous_database_names(
        &self,
        conn: &mut MySqlConnection,
    ) -> Result<Vec<String>, QueryError> {
        conn.fetch_all(mysql::GET_DATABASE_NAMES)
            .await?
            .iter()
            .map(|row| row.try_get(0))
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    async fn create_entities(&self, db_name: &str) -> Result<(), ConnectionError> {
        let opts = self.privileged_opts.clone().database(db_name);
        let conn = MySqlConnection::connect_with(&opts).await?;
        (self.create_entities)(conn).await;
        Ok(())
    }

    async fn create_connection_pool(&self, db_id: Uuid) -> Result<MySqlPool, BuildError> {
        let db_name = get_db_name(db_id);
        let db_name = db_name.as_str();
        let opts = self
            .privileged_opts
            .clone()
            .database(db_name)
            .username(db_name)
            .password(db_name);
        let pool = (self.create_restricted_pool)().connect_lazy_with(opts);
        Ok(pool)
    }

    async fn get_table_names(
        &self,
        db_name: &str,
        conn: &mut MySqlConnection,
    ) -> Result<Vec<String>, QueryError> {
        conn.fetch_all(mysql::get_table_names(db_name).as_str())
            .await?
            .iter()
            .map(|row| row.try_get(0))
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    fn get_drop_previous_databases(&self) -> bool {
        self.drop_previous_databases_flag
    }
}

type BError = BackendError<BuildError, PoolError, ConnectionError, QueryError>;

#[async_trait]
impl Backend for SqlxMySQLBackend {
    type Pool = MySqlPool;

    type BuildError = BuildError;
    type PoolError = PoolError;
    type ConnectionError = ConnectionError;
    type QueryError = QueryError;

    async fn init(&self) -> Result<(), BError> {
        MySQLBackendWrapper::new(self).init().await
    }

    async fn create(
        &self,
        db_id: uuid::Uuid,
        restrict_privileges: bool,
    ) -> Result<MySqlPool, BError> {
        MySQLBackendWrapper::new(self)
            .create(db_id, restrict_privileges)
            .await
    }

    async fn clean(&self, db_id: uuid::Uuid) -> Result<(), BError> {
        MySQLBackendWrapper::new(self).clean(db_id).await
    }

    async fn drop(&self, db_id: uuid::Uuid, _is_restricted: bool) -> Result<(), BError> {
        MySQLBackendWrapper::new(self).drop(db_id).await
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::needless_return)]

    use futures::{future::join_all, StreamExt};
    use sqlx::{
        mysql::{MySqlConnectOptions, MySqlPoolOptions},
        query, query_as, Executor, FromRow, Row,
    };
    use tokio_shared_rt::test;

    use crate::{
        common::statement::mysql::tests::{
            CREATE_ENTITIES_STATEMENTS, DDL_STATEMENTS, DML_STATEMENTS,
        },
        r#async::{
            backend::mysql::r#trait::tests::test_backend_creates_database_with_unrestricted_privileges,
            db_pool::DatabasePoolBuilder,
        },
        tests::get_privileged_mysql_config,
    };

    use super::{
        super::r#trait::tests::{
            test_backend_cleans_database_with_tables, test_backend_cleans_database_without_tables,
            test_backend_creates_database_with_restricted_privileges, test_backend_drops_database,
            test_backend_drops_previous_databases, test_pool_drops_created_restricted_databases,
            test_pool_drops_created_unrestricted_database, test_pool_drops_previous_databases,
            MySQLDropLock,
        },
        SqlxMySQLBackend,
    };

    fn create_backend(with_table: bool) -> SqlxMySQLBackend {
        let config = get_privileged_mysql_config();
        let opts = MySqlConnectOptions::new().username(config.username.as_str());
        let opts = if let Some(password) = &config.password {
            opts.password(password)
        } else {
            opts
        };
        SqlxMySQLBackend::new(opts, MySqlPoolOptions::new, MySqlPoolOptions::new, {
            move |mut conn| {
                if with_table {
                    Box::pin(async move {
                        conn.execute_many(CREATE_ENTITIES_STATEMENTS.join(";").as_str())
                            .collect::<Vec<_>>()
                            .await
                            .drain(..)
                            .collect::<Result<Vec<_>, _>>()
                            .unwrap();
                    })
                } else {
                    Box::pin(async {})
                }
            }
        })
    }

    #[test(flavor = "multi_thread", shared)]
    async fn backend_drops_previous_databases() {
        test_backend_drops_previous_databases(
            create_backend(false),
            create_backend(false).drop_previous_databases(true),
            create_backend(false).drop_previous_databases(false),
        )
        .await;
    }

    #[test(flavor = "multi_thread", shared)]
    async fn backend_creates_database_with_restricted_privileges() {
        let backend = create_backend(true).drop_previous_databases(false);
        test_backend_creates_database_with_restricted_privileges(backend).await;
    }

    #[test(flavor = "multi_thread", shared)]
    async fn backend_creates_database_with_unrestricted_privileges() {
        let backend = create_backend(true).drop_previous_databases(false);
        test_backend_creates_database_with_unrestricted_privileges(backend).await;
    }

    #[test(flavor = "multi_thread", shared)]
    async fn backend_cleans_database_with_tables() {
        let backend = create_backend(true).drop_previous_databases(false);
        test_backend_cleans_database_with_tables(backend).await;
    }

    #[test(flavor = "multi_thread", shared)]
    async fn backend_cleans_database_without_tables() {
        let backend = create_backend(false).drop_previous_databases(false);
        test_backend_cleans_database_without_tables(backend).await;
    }

    #[test(flavor = "multi_thread", shared)]
    async fn backend_drops_restricted_database() {
        let backend = create_backend(true).drop_previous_databases(false);
        test_backend_drops_database(backend, true).await;
    }

    #[test(flavor = "multi_thread", shared)]
    async fn backend_drops_unrestricted_database() {
        let backend = create_backend(true).drop_previous_databases(false);
        test_backend_drops_database(backend, false).await;
    }

    #[test(flavor = "multi_thread", shared)]
    async fn pool_drops_previous_databases() {
        test_pool_drops_previous_databases(
            create_backend(false),
            create_backend(false).drop_previous_databases(true),
            create_backend(false).drop_previous_databases(false),
        )
        .await;
    }

    #[test(flavor = "multi_thread", shared)]
    async fn pool_provides_isolated_databases() {
        #[derive(FromRow, Eq, PartialEq, Debug)]
        struct Book {
            title: String,
        }

        const NUM_DBS: i64 = 3;

        let backend = create_backend(true).drop_previous_databases(false);

        async {
            let db_pool = backend.create_database_pool().await.unwrap();
            let conn_pools = join_all((0..NUM_DBS).map(|_| db_pool.pull_immutable())).await;

            // insert single row into each database
            join_all(
                conn_pools
                    .iter()
                    .enumerate()
                    .map(|(i, conn_pool)| async move {
                        query("INSERT INTO book (title) VALUES (?)")
                            .bind(format!("Title {i}"))
                            .execute(&***conn_pool)
                            .await
                            .unwrap();
                    }),
            )
            .await;

            // rows fetched must be as inserted
            join_all(
                conn_pools
                    .iter()
                    .enumerate()
                    .map(|(i, conn_pool)| async move {
                        assert_eq!(
                            query_as::<_, Book>("SELECT title FROM book")
                                .fetch_all(&***conn_pool)
                                .await
                                .unwrap(),
                            vec![Book {
                                title: format!("Title {i}")
                            }]
                        );
                    }),
            )
            .await;
        }
        .lock_read()
        .await;
    }

    #[test(flavor = "multi_thread", shared)]
    async fn pool_provides_restricted_databases() {
        let backend = create_backend(true).drop_previous_databases(false);

        async {
            let db_pool = backend.create_database_pool().await.unwrap();

            let conn_pool = db_pool.pull_immutable().await;
            let conn = &mut conn_pool.acquire().await.unwrap();

            // DDL statements must fail
            for stmt in DDL_STATEMENTS {
                assert!(conn.execute(stmt).await.is_err());
            }

            // DML statements must succeed
            for stmt in DML_STATEMENTS {
                assert!(conn.execute(stmt).await.is_ok());
            }
        }
        .lock_read()
        .await;
    }

    #[test(flavor = "multi_thread", shared)]
    async fn pool_provides_unrestricted_databases() {
        let backend = create_backend(true).drop_previous_databases(false);

        async {
            let db_pool = backend.create_database_pool().await.unwrap();

            // DML statements must succeed
            {
                let conn_pool = db_pool.create_mutable().await.unwrap();
                let conn = &mut conn_pool.acquire().await.unwrap();
                for stmt in DML_STATEMENTS {
                    assert!(conn.execute(stmt).await.is_ok());
                }
            }

            // DDL statements must succeed
            for stmt in DDL_STATEMENTS {
                let conn_pool = db_pool.create_mutable().await.unwrap();
                let conn = &mut conn_pool.acquire().await.unwrap();
                assert!(conn.execute(stmt).await.is_ok());
            }
        }
        .lock_read()
        .await;
    }

    #[test(flavor = "multi_thread", shared)]
    async fn pool_provides_clean_databases() {
        const NUM_DBS: i64 = 3;

        let backend = create_backend(true).drop_previous_databases(false);

        async {
            let db_pool = backend.create_database_pool().await.unwrap();

            // fetch connection pools the first time
            {
                let conn_pools = join_all((0..NUM_DBS).map(|_| db_pool.pull_immutable())).await;

                // databases must be empty
                join_all(conn_pools.iter().map(|conn_pool| async move {
                    assert_eq!(
                        query("SELECT COUNT(*) FROM book")
                            .fetch_one(&***conn_pool)
                            .await
                            .unwrap()
                            .get::<i64, _>(0),
                        0
                    );
                }))
                .await;

                // insert data into each database
                join_all(conn_pools.iter().map(|conn_pool| async move {
                    query("INSERT INTO book (title) VALUES (?)")
                        .bind("Title")
                        .execute(&***conn_pool)
                        .await
                        .unwrap();
                }))
                .await;
            }

            // fetch same connection pools a second time
            {
                let conn_pools = join_all((0..NUM_DBS).map(|_| db_pool.pull_immutable())).await;

                // databases must be empty
                join_all(conn_pools.iter().map(|conn_pool| async move {
                    assert_eq!(
                        query("SELECT COUNT(*) FROM book")
                            .fetch_one(&***conn_pool)
                            .await
                            .unwrap()
                            .get::<i64, _>(0),
                        0
                    );
                }))
                .await;
            }
        }
        .lock_read()
        .await;
    }

    #[test(flavor = "multi_thread", shared)]
    async fn pool_drops_created_restricted_databases() {
        let backend = create_backend(false);
        test_pool_drops_created_restricted_databases(backend).await;
    }

    #[test(flavor = "multi_thread", shared)]
    async fn pool_drops_created_unrestricted_databases() {
        let backend = create_backend(false);
        test_pool_drops_created_unrestricted_database(backend).await;
    }
}
