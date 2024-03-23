use std::{borrow::Cow, collections::HashMap, convert::Into, pin::Pin};

use async_trait::async_trait;
use futures::Future;
use parking_lot::Mutex;
use tokio_postgres::{Client, Config, NoTls};
use uuid::Uuid;

use crate::{common::statement::postgres, util::get_db_name};

use super::{
    super::{
        common::{
            error::tokio_postgres::{ConnectionError, QueryError},
            pool::tokio_postgres::r#trait::TokioPostgresPoolAssociation,
        },
        error::Error as BackendError,
        r#trait::Backend,
    },
    r#trait::{PostgresBackend, PostgresBackendWrapper},
};

type CreateEntities = dyn Fn(Client) -> Pin<Box<dyn Future<Output = Client> + Send + 'static>>
    + Send
    + Sync
    + 'static;

/// ``tokio-postgres`` backend
pub struct TokioPostgresBackend<P>
where
    P: TokioPostgresPoolAssociation,
{
    privileged_config: Config,
    default_pool: P::Pool,
    db_conns: Mutex<HashMap<Uuid, Client>>,
    create_restricted_pool: Box<dyn Fn() -> P::Builder + Send + Sync + 'static>,
    create_entities: Box<CreateEntities>,
    drop_previous_databases_flag: bool,
}

impl<P> TokioPostgresBackend<P>
where
    P: TokioPostgresPoolAssociation,
{
    /// Creates a new ``tokio-postgres`` backend
    /// # Example
    /// ```
    /// use bb8::Pool;
    /// use db_pool::{
    ///     r#async::{TokioPostgresBackend, TokioPostgresBb8},
    ///     PrivilegedPostgresConfig,
    /// };
    /// use dotenvy::dotenv;
    ///
    /// async fn f() {
    ///     dotenv().ok();
    ///
    ///     let config = PrivilegedPostgresConfig::from_env().unwrap();
    ///     
    ///     let backend = TokioPostgresBackend::<TokioPostgresBb8>::new(
    ///         config.into(),
    ///         || Pool::builder().max_size(10),
    ///         || Pool::builder().max_size(2),
    ///         move |conn| {
    ///             Box::pin(async move {
    ///                 conn.execute(
    ///                     "CREATE TABLE book(id SERIAL PRIMARY KEY, title TEXT NOT NULL)",
    ///                     &[],
    ///                 )
    ///                 .await
    ///                 .unwrap();
    ///                 conn
    ///             })
    ///         },
    ///     )
    ///     .await
    ///     .unwrap();
    /// }
    ///
    /// tokio_test::block_on(f());
    /// ```
    pub async fn new(
        privileged_config: Config,
        create_privileged_pool: impl Fn() -> P::Builder,
        create_restricted_pool: impl Fn() -> P::Builder + Send + Sync + 'static,
        create_entities: impl Fn(Client) -> Pin<Box<dyn Future<Output = Client> + Send + 'static>>
            + Send
            + Sync
            + 'static,
    ) -> Result<Self, P::BuildError> {
        let builder = create_privileged_pool();
        let default_pool = P::build_pool(builder, privileged_config.clone()).await?;

        Ok(Self {
            privileged_config,
            default_pool,
            db_conns: Mutex::new(HashMap::new()),
            create_entities: Box::new(create_entities),
            create_restricted_pool: Box::new(create_restricted_pool),
            drop_previous_databases_flag: true,
        })
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
impl<'pool, P> PostgresBackend<'pool> for TokioPostgresBackend<P>
where
    P: TokioPostgresPoolAssociation,
{
    type Connection = Client;
    type PooledConnection = P::PooledConnection<'pool>;
    type Pool = P::Pool;

    type BuildError = P::BuildError;
    type PoolError = P::PoolError;
    type ConnectionError = ConnectionError;
    type QueryError = QueryError;

    async fn execute_stmt(&self, query: &str, conn: &mut Client) -> Result<(), QueryError> {
        conn.execute(query, &[]).await?;
        Ok(())
    }

    async fn batch_execute_stmt<'a>(
        &self,
        query: impl IntoIterator<Item = Cow<'a, str>> + Send,
        conn: &mut Client,
    ) -> Result<(), QueryError> {
        let query = query.into_iter().collect::<Vec<_>>().join(";");
        conn.batch_execute(query.as_str()).await?;
        Ok(())
    }

    async fn get_default_connection(
        &'pool self,
    ) -> Result<P::PooledConnection<'pool>, P::PoolError> {
        P::get_connection(&self.default_pool).await
    }

    async fn establish_database_connection(&self, db_id: Uuid) -> Result<Client, ConnectionError> {
        let mut config = self.privileged_config.clone();
        let db_name = get_db_name(db_id);
        config.dbname(db_name.as_str());
        let (client, connection) = config.connect(NoTls).await?;
        tokio::spawn(connection);
        Ok(client)
    }

    fn put_database_connection(&self, db_id: Uuid, conn: Client) {
        self.db_conns.lock().insert(db_id, conn);
    }

    fn get_database_connection(&self, db_id: Uuid) -> Client {
        self.db_conns
            .lock()
            .remove(&db_id)
            .unwrap_or_else(|| panic!("connection map must have a connection for {db_id}"))
    }

    async fn get_previous_database_names(
        &self,
        conn: &mut Client,
    ) -> Result<Vec<String>, QueryError> {
        conn.query(postgres::GET_DATABASE_NAMES, &[])
            .await
            .map(|rows| rows.iter().map(|row| row.get(0)).collect())
            .map_err(Into::into)
    }

    async fn create_entities(&self, conn: Client) -> Client {
        (self.create_entities)(conn).await
    }

    async fn create_connection_pool(&self, db_id: Uuid) -> Result<P::Pool, P::BuildError> {
        let db_name = get_db_name(db_id);
        let db_name = db_name.as_str();
        let mut config = self.privileged_config.clone();
        config.dbname(db_name);
        config.user(db_name);
        config.password(db_name);
        let builder = (self.create_restricted_pool)();
        P::build_pool(builder, config).await
    }

    async fn get_table_names(
        &self,
        privileged_conn: &mut Client,
    ) -> Result<Vec<String>, QueryError> {
        privileged_conn
            .query(postgres::GET_TABLE_NAMES, &[])
            .await
            .map(|rows| rows.iter().map(|row| row.get(0)).collect())
            .map_err(Into::into)
    }

    fn get_drop_previous_databases(&self) -> bool {
        self.drop_previous_databases_flag
    }
}

type BError<BuildError, PoolError> =
    BackendError<BuildError, PoolError, ConnectionError, QueryError>;

#[async_trait]
impl<P> Backend for TokioPostgresBackend<P>
where
    P: TokioPostgresPoolAssociation,
{
    type Pool = P::Pool;

    type BuildError = P::BuildError;
    type PoolError = P::PoolError;
    type ConnectionError = ConnectionError;
    type QueryError = QueryError;

    async fn init(&self) -> Result<(), BError<P::BuildError, P::PoolError>> {
        PostgresBackendWrapper::new(self).init().await
    }

    async fn create(
        &self,
        db_id: uuid::Uuid,
    ) -> Result<P::Pool, BError<P::BuildError, P::PoolError>> {
        PostgresBackendWrapper::new(self).create(db_id).await
    }

    async fn clean(&self, db_id: uuid::Uuid) -> Result<(), BError<P::BuildError, P::PoolError>> {
        PostgresBackendWrapper::new(self).clean(db_id).await
    }

    async fn drop(&self, db_id: uuid::Uuid) -> Result<(), BError<P::BuildError, P::PoolError>> {
        PostgresBackendWrapper::new(self).drop(db_id).await
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::needless_return)]

    use bb8::Pool;
    use futures::future::join_all;
    use tokio_postgres::Config;
    use tokio_shared_rt::test;

    use crate::{
        common::statement::postgres::tests::{
            CREATE_ENTITIES_STATEMENT, DDL_STATEMENTS, DML_STATEMENTS,
        },
        r#async::{
            backend::common::pool::tokio_postgres::bb8::TokioPostgresBb8,
            db_pool::DatabasePoolBuilder,
        },
    };

    use super::{
        super::r#trait::tests::{
            test_backend_cleans_database_with_tables, test_backend_cleans_database_without_tables,
            test_backend_creates_database_with_restricted_privileges, test_backend_drops_database,
            test_backend_drops_previous_databases, test_pool_drops_created_databases,
            test_pool_drops_previous_databases, PgDropLock,
        },
        TokioPostgresBackend,
    };

    async fn create_backend(with_table: bool) -> TokioPostgresBackend<TokioPostgresBb8> {
        let mut config = Config::new();
        config
            .host("localhost")
            .user("postgres")
            .password("postgres");
        TokioPostgresBackend::new(config, Pool::builder, Pool::builder, {
            move |conn| {
                if with_table {
                    Box::pin(async move {
                        conn.execute(CREATE_ENTITIES_STATEMENT, &[]).await.unwrap();
                        conn
                    })
                } else {
                    Box::pin(async { conn })
                }
            }
        })
        .await
        .unwrap()
    }

    #[test(flavor = "multi_thread", shared)]
    async fn backend_drops_previous_databases() {
        test_backend_drops_previous_databases(
            create_backend(false).await,
            create_backend(false).await.drop_previous_databases(true),
            create_backend(false).await.drop_previous_databases(false),
        )
        .await;
    }

    #[test(flavor = "multi_thread", shared)]
    async fn backend_creates_database_with_restricted_privileges() {
        let backend = create_backend(true).await.drop_previous_databases(false);
        test_backend_creates_database_with_restricted_privileges(backend).await;
    }

    #[test(flavor = "multi_thread", shared)]
    async fn backend_cleans_database_with_tables() {
        let backend = create_backend(true).await.drop_previous_databases(false);
        test_backend_cleans_database_with_tables(backend).await;
    }

    #[test(flavor = "multi_thread", shared)]
    async fn backend_cleans_database_without_tables() {
        let backend = create_backend(false).await.drop_previous_databases(false);
        test_backend_cleans_database_without_tables(backend).await;
    }

    #[test(flavor = "multi_thread", shared)]
    async fn backend_drops_database() {
        let backend = create_backend(true).await.drop_previous_databases(false);
        test_backend_drops_database(backend).await;
    }

    #[test(flavor = "multi_thread", shared)]
    async fn pool_drops_previous_databases() {
        test_pool_drops_previous_databases(
            create_backend(false).await,
            create_backend(false).await.drop_previous_databases(true),
            create_backend(false).await.drop_previous_databases(false),
        )
        .await;
    }

    #[test(flavor = "multi_thread", shared)]
    async fn pool_provides_isolated_databases() {
        const NUM_DBS: i64 = 3;

        let backend = create_backend(true).await.drop_previous_databases(false);

        async {
            let db_pool = backend.create_database_pool().await.unwrap();
            let conn_pools = join_all((0..NUM_DBS).map(|_| db_pool.pull())).await;

            // insert single row into each database
            join_all(
                conn_pools
                    .iter()
                    .enumerate()
                    .map(|(i, conn_pool)| async move {
                        let conn = &mut conn_pool.get().await.unwrap();
                        conn.execute(
                            "INSERT INTO book (title) VALUES ($1)",
                            &[&format!("Title {i}").as_str()],
                        )
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
                        let conn = &mut conn_pool.get().await.unwrap();
                        assert_eq!(
                            conn.query("SELECT title FROM book", &[])
                                .await
                                .unwrap()
                                .iter()
                                .map(|row| row.get::<_, String>(0))
                                .collect::<Vec<_>>(),
                            vec![format!("Title {i}")]
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
        let backend = create_backend(true).await.drop_previous_databases(false);

        async {
            let db_pool = backend.create_database_pool().await.unwrap();

            let conn_pool = db_pool.pull().await;
            let conn = &mut conn_pool.get().await.unwrap();

            // DDL statements must fail
            for stmt in DDL_STATEMENTS {
                assert!(conn.execute(stmt, &[]).await.is_err());
            }

            // DML statements must succeed
            for stmt in DML_STATEMENTS {
                assert!(conn.execute(stmt, &[]).await.is_ok());
            }
        }
        .lock_read()
        .await;
    }

    #[test(flavor = "multi_thread", shared)]
    async fn pool_provides_clean_databases() {
        const NUM_DBS: i64 = 3;

        let backend = create_backend(true).await.drop_previous_databases(false);

        async {
            let db_pool = backend.create_database_pool().await.unwrap();

            // fetch connection pools the first time
            {
                let conn_pools = join_all((0..NUM_DBS).map(|_| db_pool.pull())).await;

                // databases must be empty
                join_all(conn_pools.iter().map(|conn_pool| async move {
                    let conn = &mut conn_pool.get().await.unwrap();
                    assert_eq!(
                        conn.query_one("SELECT COUNT(*) FROM book", &[])
                            .await
                            .unwrap()
                            .get::<_, i64>(0),
                        0
                    );
                }))
                .await;

                // insert data into each database
                join_all(conn_pools.iter().map(|conn_pool| async move {
                    let conn = &mut conn_pool.get().await.unwrap();
                    conn.execute("INSERT INTO book (title) VALUES ($1)", &[&"Title"])
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
                    let conn = &mut conn_pool.get().await.unwrap();
                    assert_eq!(
                        conn.query_one("SELECT COUNT(*) FROM book", &[])
                            .await
                            .unwrap()
                            .get::<_, i64>(0),
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
        let backend = create_backend(false).await;
        test_pool_drops_created_databases(backend).await;
    }
}
