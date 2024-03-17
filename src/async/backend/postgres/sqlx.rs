use std::{borrow::Cow, collections::HashMap, pin::Pin};

use async_trait::async_trait;
use futures::Future;
use parking_lot::Mutex;
use sqlx::{
    pool::PoolConnection,
    postgres::{PgConnectOptions, PgPoolOptions},
    Connection, Executor, PgConnection, PgPool, Postgres, Row,
};
use uuid::Uuid;

use crate::{common::statement::postgres, util::get_db_name};

use super::{
    super::{
        common::error::sqlx::{BuildError, ConnectionError, PoolError, QueryError},
        error::Error as BackendError,
        r#trait::Backend,
    },
    r#trait::{PostgresBackend, PostgresBackendWrapper},
};

type CreateEntities = dyn Fn(PgConnection) -> Pin<Box<dyn Future<Output = PgConnection> + Send + 'static>>
    + Send
    + Sync
    + 'static;

pub struct SqlxPostgresBackend {
    privileged_opts: PgConnectOptions,
    default_pool: PgPool,
    db_conns: Mutex<HashMap<Uuid, PgConnection>>,
    create_restricted_pool: Box<dyn Fn() -> PgPoolOptions + Send + Sync + 'static>,
    create_entities: Box<CreateEntities>,
    drop_previous_databases_flag: bool,
}

impl SqlxPostgresBackend {
    pub fn new(
        privileged_options: PgConnectOptions,
        create_privileged_pool: impl Fn() -> PgPoolOptions,
        create_restricted_pool: impl Fn() -> PgPoolOptions + Send + Sync + 'static,
        create_entities: impl Fn(PgConnection) -> Pin<Box<dyn Future<Output = PgConnection> + Send + 'static>>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        let pool_opts = create_privileged_pool();
        let default_pool = pool_opts.connect_lazy_with(privileged_options.clone());

        Self {
            privileged_opts: privileged_options,
            default_pool,
            db_conns: Mutex::new(HashMap::new()),
            create_restricted_pool: Box::new(create_restricted_pool),
            create_entities: Box::new(create_entities),
            drop_previous_databases_flag: true,
        }
    }

    #[must_use]
    pub fn drop_previous_databases(self, value: bool) -> Self {
        Self {
            drop_previous_databases_flag: value,
            ..self
        }
    }
}

#[async_trait]
impl<'pool> PostgresBackend<'pool> for SqlxPostgresBackend {
    type Connection = PgConnection;
    type PooledConnection = PoolConnection<Postgres>;
    type Pool = PgPool;

    type BuildError = BuildError;
    type PoolError = PoolError;
    type ConnectionError = ConnectionError;
    type QueryError = QueryError;

    async fn execute_stmt(&self, query: &str, conn: &mut PgConnection) -> Result<(), QueryError> {
        conn.execute(query).await?;
        Ok(())
    }

    async fn batch_execute_stmt<'a>(
        &self,
        query: impl IntoIterator<Item = Cow<'a, str>> + Send,
        conn: &mut PgConnection,
    ) -> Result<(), QueryError> {
        let query = query.into_iter().collect::<Vec<_>>().join(";");
        self.execute_stmt(query.as_str(), conn).await
    }

    async fn get_default_connection(&'pool self) -> Result<PoolConnection<Postgres>, PoolError> {
        self.default_pool.acquire().await.map_err(Into::into)
    }

    async fn establish_database_connection(
        &self,
        db_id: Uuid,
    ) -> Result<PgConnection, ConnectionError> {
        let db_name = get_db_name(db_id);
        let opts = self.privileged_opts.clone().database(db_name.as_str());
        PgConnection::connect_with(&opts).await.map_err(Into::into)
    }

    fn put_database_connection(&self, db_id: Uuid, conn: PgConnection) {
        self.db_conns.lock().insert(db_id, conn);
    }

    fn get_database_connection(&self, db_id: Uuid) -> PgConnection {
        self.db_conns
            .lock()
            .remove(&db_id)
            .unwrap_or_else(|| panic!("connection map must have a connection for {db_id}"))
    }

    async fn get_previous_database_names(
        &self,
        conn: &mut PgConnection,
    ) -> Result<Vec<String>, QueryError> {
        conn.fetch_all(postgres::GET_DATABASE_NAMES)
            .await?
            .iter()
            .map(|row| row.try_get(0))
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    async fn create_entities(&self, conn: PgConnection) -> PgConnection {
        (self.create_entities)(conn).await
    }

    async fn create_connection_pool(&self, db_id: Uuid) -> Result<PgPool, BuildError> {
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

    async fn get_table_names(&self, conn: &mut PgConnection) -> Result<Vec<String>, QueryError> {
        conn.fetch_all(postgres::GET_TABLE_NAMES)
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
impl Backend for SqlxPostgresBackend {
    type Pool = PgPool;

    type BuildError = BuildError;
    type PoolError = PoolError;
    type ConnectionError = ConnectionError;
    type QueryError = QueryError;

    async fn init(&self) -> Result<(), BError> {
        PostgresBackendWrapper::new(self).init().await
    }

    async fn create(&self, db_id: uuid::Uuid) -> Result<PgPool, BError> {
        PostgresBackendWrapper::new(self).create(db_id).await
    }

    async fn clean(&self, db_id: uuid::Uuid) -> Result<(), BError> {
        PostgresBackendWrapper::new(self).clean(db_id).await
    }

    async fn drop(&self, db_id: uuid::Uuid) -> Result<(), BError> {
        PostgresBackendWrapper::new(self).drop(db_id).await
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::needless_return)]

    use futures::future::join_all;
    use sqlx::{
        postgres::{PgConnectOptions, PgPoolOptions},
        query, query_as, Executor, FromRow, Row,
    };
    use tokio_shared_rt::test;

    use crate::{
        common::statement::postgres::tests::{
            CREATE_ENTITIES_STATEMENT, DDL_STATEMENTS, DML_STATEMENTS,
        },
        r#async::db_pool::DatabasePoolBuilder,
    };

    use super::{
        super::r#trait::tests::{
            test_backend_cleans_database, test_backend_creates_database_with_restricted_privileges,
            test_backend_drops_database, test_backend_drops_previous_databases,
            test_pool_drops_created_databases, test_pool_drops_previous_databases, DropLock,
        },
        SqlxPostgresBackend,
    };

    fn create_backend(with_table: bool) -> SqlxPostgresBackend {
        SqlxPostgresBackend::new(
            PgConnectOptions::new()
                .username("postgres")
                .password("postgres"),
            PgPoolOptions::new,
            PgPoolOptions::new,
            {
                move |mut conn| {
                    if with_table {
                        Box::pin(async move {
                            conn.execute(CREATE_ENTITIES_STATEMENT).await.unwrap();
                            conn
                        })
                    } else {
                        Box::pin(async { conn })
                    }
                }
            },
        )
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
    async fn backend_cleans_database() {
        let backend = create_backend(true).drop_previous_databases(false);
        test_backend_cleans_database(backend).await;
    }

    #[test(flavor = "multi_thread", shared)]
    async fn backend_drops_database() {
        let backend = create_backend(true).drop_previous_databases(false);
        test_backend_drops_database(backend).await;
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
            let conn_pools = join_all((0..NUM_DBS).map(|_| db_pool.pull())).await;

            // insert single row into each database
            join_all(
                conn_pools
                    .iter()
                    .enumerate()
                    .map(|(i, conn_pool)| async move {
                        query("INSERT INTO book (title) VALUES ($1)")
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

            let conn_pool = db_pool.pull().await;
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
    async fn pool_provides_clean_databases() {
        const NUM_DBS: i64 = 3;

        let backend = create_backend(true).drop_previous_databases(false);

        async {
            let db_pool = backend.create_database_pool().await.unwrap();

            // fetch connection pools the first time
            {
                let conn_pools = join_all((0..NUM_DBS).map(|_| db_pool.pull())).await;

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
                    query("INSERT INTO book (title) VALUES ($1)")
                        .bind("Title")
                        .execute(&***conn_pool)
                        .await
                        .unwrap();
                }))
                .await;
            }

            // fetch same connection pools a second time
            {
                let conn_pools = join_all((0..NUM_DBS).map(|_| db_pool.pull())).await;

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
    async fn pool_drops_created_databases() {
        let backend = create_backend(false);
        test_pool_drops_created_databases(backend).await;
    }
}
