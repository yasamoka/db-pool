use std::{borrow::Cow, pin::Pin};

use async_trait::async_trait;
use diesel::{prelude::*, result::Error, sql_query, table};
use diesel_async::{
    AsyncConnection, AsyncMysqlConnection, RunQueryDsl, SimpleAsyncConnection,
    pooled_connection::{AsyncDieselConnectionManager, ManagerConfig, SetupCallback},
};
use futures::{Future, future::FutureExt};
use uuid::Uuid;

use crate::{
    common::{config::mysql::PrivilegedMySQLConfig, statement::mysql},
    util::get_db_name,
};

use super::{
    super::{
        common::pool::diesel::r#trait::DieselPoolAssociation, error::Error as BackendError,
        r#trait::Backend,
    },
    r#trait::{MySQLBackend, MySQLBackendWrapper},
};

type CreateEntities = dyn Fn(AsyncMysqlConnection) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
    + Send
    + Sync
    + 'static;

/// [`Diesel async MySQL`](https://docs.rs/diesel-async/0.5.2/diesel_async/struct.AsyncMysqlConnection.html) backend
pub struct DieselAsyncMySQLBackend<P: DieselPoolAssociation<AsyncMysqlConnection>> {
    privileged_config: PrivilegedMySQLConfig,
    default_pool: P::Pool,
    create_restricted_pool: Box<
        dyn Fn(AsyncDieselConnectionManager<AsyncMysqlConnection>) -> P::Builder
            + Send
            + Sync
            + 'static,
    >,
    create_connection: Box<dyn Fn() -> SetupCallback<AsyncMysqlConnection> + Send + Sync + 'static>,
    create_entities: Box<CreateEntities>,
    drop_previous_databases_flag: bool,
}

impl<P: DieselPoolAssociation<AsyncMysqlConnection>> DieselAsyncMySQLBackend<P> {
    /// Creates a new [`Diesel async MySQL`](https://docs.rs/diesel-async/0.5.2/diesel_async/struct.AsyncMysqlConnection.html) backend
    /// # Example
    /// ```
    /// use bb8::Pool;
    /// use db_pool::{
    ///     r#async::{DieselAsyncMySQLBackend, DieselBb8},
    ///     PrivilegedMySQLConfig,
    /// };
    /// use diesel::sql_query;
    /// use diesel_async::RunQueryDsl;
    /// use dotenvy::dotenv;
    ///
    /// async fn f() {
    ///     dotenv().ok();
    ///
    ///     let config = PrivilegedMySQLConfig::from_env().unwrap();
    ///
    ///     let backend = DieselAsyncMySQLBackend::<DieselBb8>::new(
    ///         config,
    ///         |_| Pool::builder().max_size(10),
    ///         |_| Pool::builder().max_size(2),
    ///         None,
    ///         move |mut conn| {
    ///             Box::pin(async move {
    ///                 sql_query("CREATE TABLE book(id INTEGER PRIMARY KEY AUTO_INCREMENT, title TEXT NOT NULL)")
    ///                     .execute(&mut conn)
    ///                     .await
    ///                     .unwrap();
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
        privileged_config: PrivilegedMySQLConfig,
        create_privileged_pool: impl Fn(
            AsyncDieselConnectionManager<AsyncMysqlConnection>,
        ) -> P::Builder,
        create_restricted_pool: impl Fn(
            AsyncDieselConnectionManager<AsyncMysqlConnection>,
        ) -> P::Builder
        + Send
        + Sync
        + 'static,
        custom_create_connection: Option<
            Box<dyn Fn() -> SetupCallback<AsyncMysqlConnection> + Send + Sync + 'static>,
        >,
        create_entities: impl Fn(
            AsyncMysqlConnection,
        ) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
        + Send
        + Sync
        + 'static,
    ) -> Result<Self, P::BuildError> {
        let create_connection = custom_create_connection.unwrap_or_else(|| {
            Box::new(|| {
                Box::new(|connection_url| AsyncMysqlConnection::establish(connection_url).boxed())
            })
        });

        let manager = || {
            let manager_config = {
                let mut config = ManagerConfig::default();
                config.custom_setup = Box::new(create_connection());
                config
            };
            AsyncDieselConnectionManager::new_with_config(
                privileged_config.default_connection_url(),
                manager_config,
            )
        };
        let builder = create_privileged_pool(manager());
        let default_pool = P::build_pool(builder, manager()).await?;

        Ok(Self {
            privileged_config,
            default_pool,
            create_restricted_pool: Box::new(create_restricted_pool),
            create_connection: Box::new(create_connection),
            create_entities: Box::new(create_entities),
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
impl<'pool, P: DieselPoolAssociation<AsyncMysqlConnection>> MySQLBackend<'pool>
    for DieselAsyncMySQLBackend<P>
{
    type Connection = AsyncMysqlConnection;
    type PooledConnection = P::PooledConnection<'pool>;
    type Pool = P::Pool;

    type BuildError = P::BuildError;
    type PoolError = P::PoolError;
    type ConnectionError = ConnectionError;
    type QueryError = Error;

    async fn get_connection(&'pool self) -> Result<P::PooledConnection<'pool>, P::PoolError> {
        P::get_connection(&self.default_pool).await
    }

    async fn execute_query(&self, query: &str, conn: &mut AsyncMysqlConnection) -> QueryResult<()> {
        sql_query(query).execute(conn).await?;
        Ok(())
    }

    async fn batch_execute_query<'a>(
        &self,
        query: impl IntoIterator<Item = Cow<'a, str>> + Send,
        conn: &mut AsyncMysqlConnection,
    ) -> QueryResult<()> {
        let query = query.into_iter().collect::<Vec<_>>();
        if query.is_empty() {
            Ok(())
        } else {
            conn.batch_execute(query.join(";").as_str()).await
        }
    }

    fn get_host(&self) -> &str {
        self.privileged_config.host.as_str()
    }

    async fn get_previous_database_names(
        &self,
        conn: &mut AsyncMysqlConnection,
    ) -> QueryResult<Vec<String>> {
        table! {
            schemata (schema_name) {
                schema_name -> Text
            }
        }

        schemata::table
            .select(schemata::schema_name)
            .filter(schemata::schema_name.like("db_pool_%"))
            .load::<String>(conn)
            .await
    }

    async fn create_entities(&self, db_name: &str) -> Result<(), ConnectionError> {
        let database_url = self
            .privileged_config
            .privileged_database_connection_url(db_name);
        let conn = (self.create_connection)()(database_url.as_str()).await?;
        (self.create_entities)(conn).await;
        Ok(())
    }

    async fn create_connection_pool(&self, db_id: Uuid) -> Result<P::Pool, P::BuildError> {
        let db_name = get_db_name(db_id);
        let db_name = db_name.as_str();
        let database_url = self.privileged_config.restricted_database_connection_url(
            db_name,
            Some(db_name),
            db_name,
        );

        let manager = || {
            let manager_config = {
                let mut config = ManagerConfig::default();
                config.custom_setup = (self.create_connection)();
                config
            };
            AsyncDieselConnectionManager::<AsyncMysqlConnection>::new_with_config(
                database_url.as_str(),
                manager_config,
            )
        };
        let builder = (self.create_restricted_pool)(manager());
        P::build_pool(builder, manager()).await
    }

    async fn get_table_names(
        &self,
        db_name: &str,
        conn: &mut AsyncMysqlConnection,
    ) -> QueryResult<Vec<String>> {
        table! {
            tables (table_name) {
                table_name -> Text,
                table_schema -> Text
            }
        }

        sql_query(mysql::USE_DEFAULT_DATABASE).execute(conn).await?;

        tables::table
            .filter(tables::table_schema.eq(db_name))
            .select(tables::table_name)
            .load::<String>(conn)
            .await
    }

    fn get_drop_previous_databases(&self) -> bool {
        self.drop_previous_databases_flag
    }
}

type BError<BuildError, PoolError> = BackendError<BuildError, PoolError, ConnectionError, Error>;

#[async_trait]
impl<P: DieselPoolAssociation<AsyncMysqlConnection>> Backend for DieselAsyncMySQLBackend<P> {
    type Pool = P::Pool;

    type BuildError = P::BuildError;
    type PoolError = P::PoolError;
    type ConnectionError = ConnectionError;
    type QueryError = Error;

    async fn init(&self) -> Result<(), BError<P::BuildError, P::PoolError>> {
        MySQLBackendWrapper::new(self).init().await
    }

    async fn create(
        &self,
        db_id: uuid::Uuid,
        restrict_privileges: bool,
    ) -> Result<P::Pool, BError<P::BuildError, P::PoolError>> {
        MySQLBackendWrapper::new(self)
            .create(db_id, restrict_privileges)
            .await
    }

    async fn clean(&self, db_id: uuid::Uuid) -> Result<(), BError<P::BuildError, P::PoolError>> {
        MySQLBackendWrapper::new(self).clean(db_id).await
    }

    async fn drop(
        &self,
        db_id: uuid::Uuid,
        _is_restricted: bool,
    ) -> Result<(), BError<P::BuildError, P::PoolError>> {
        MySQLBackendWrapper::new(self).drop(db_id).await
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::needless_return)]

    use std::borrow::Cow;

    use bb8::Pool;
    use diesel::{Insertable, QueryDsl, insert_into, sql_query, table};
    use diesel_async::{RunQueryDsl, SimpleAsyncConnection};
    use futures::future::join_all;
    use tokio_shared_rt::test;

    use crate::{
        r#async::{
            backend::{
                common::pool::diesel::bb8::DieselBb8,
                mysql::r#trait::tests::{
                    test_backend_creates_database_with_unrestricted_privileges,
                    test_pool_drops_created_unrestricted_database,
                },
            },
            db_pool::DatabasePoolBuilder,
        },
        common::statement::mysql::tests::{
            CREATE_ENTITIES_STATEMENTS, DDL_STATEMENTS, DML_STATEMENTS,
        },
        tests::get_privileged_mysql_config,
    };

    use super::{
        super::r#trait::tests::{
            MySQLDropLock, test_backend_cleans_database_with_tables,
            test_backend_cleans_database_without_tables,
            test_backend_creates_database_with_restricted_privileges, test_backend_drops_database,
            test_backend_drops_previous_databases, test_pool_drops_created_restricted_databases,
            test_pool_drops_previous_databases,
        },
        DieselAsyncMySQLBackend,
    };

    table! {
        book (id) {
            id -> Int4,
            title -> Text
        }
    }

    #[derive(Insertable)]
    #[diesel(table_name = book)]
    struct NewBook<'a> {
        title: Cow<'a, str>,
    }

    async fn create_backend(with_table: bool) -> DieselAsyncMySQLBackend<DieselBb8> {
        let config = get_privileged_mysql_config().clone();
        DieselAsyncMySQLBackend::new(config, |_| Pool::builder(), |_| Pool::builder(), None, {
            move |mut conn| {
                if with_table {
                    Box::pin(async move {
                        let query = CREATE_ENTITIES_STATEMENTS.join(";");
                        conn.batch_execute(query.as_str()).await.unwrap();
                    })
                } else {
                    Box::pin(async {})
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
    async fn backend_creates_database_with_unrestricted_privileges() {
        let backend = create_backend(true).await.drop_previous_databases(false);
        test_backend_creates_database_with_unrestricted_privileges(backend).await;
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
    async fn backend_drops_restricted_database() {
        let backend = create_backend(true).await.drop_previous_databases(false);
        test_backend_drops_database(backend, true).await;
    }

    #[test(flavor = "multi_thread", shared)]
    async fn backend_drops_unrestricted_database() {
        let backend = create_backend(true).await.drop_previous_databases(false);
        test_backend_drops_database(backend, false).await;
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
            let conn_pools = join_all((0..NUM_DBS).map(|_| db_pool.pull_immutable())).await;

            // insert single row into each database
            join_all(
                conn_pools
                    .iter()
                    .enumerate()
                    .map(|(i, conn_pool)| async move {
                        let conn = &mut conn_pool.get().await.unwrap();
                        insert_into(book::table)
                            .values(NewBook {
                                title: format!("Title {i}").into(),
                            })
                            .execute(conn)
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
                            book::table
                                .select(book::title)
                                .load::<String>(conn)
                                .await
                                .unwrap(),
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
            let conn_pool = db_pool.pull_immutable().await;
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
        .lock_read()
        .await;
    }

    #[test(flavor = "multi_thread", shared)]
    async fn pool_provides_unrestricted_databases() {
        let backend = create_backend(true).await.drop_previous_databases(false);

        async {
            let db_pool = backend.create_database_pool().await.unwrap();

            // DML statements must succeed
            {
                let conn_pool = db_pool.create_mutable().await.unwrap();
                let conn = &mut conn_pool.get().await.unwrap();
                for stmt in DML_STATEMENTS {
                    assert!(sql_query(stmt).execute(conn).await.is_ok());
                }
            }

            // DDL statements must succeed
            for stmt in DDL_STATEMENTS {
                let conn_pool = db_pool.create_mutable().await.unwrap();
                let conn = &mut conn_pool.get().await.unwrap();
                assert!(sql_query(stmt).execute(conn).await.is_ok());
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
                let conn_pools = join_all((0..NUM_DBS).map(|_| db_pool.pull_immutable())).await;

                // databases must be empty
                join_all(conn_pools.iter().map(|conn_pool| async move {
                    let conn = &mut conn_pool.get().await.unwrap();
                    assert_eq!(
                        book::table.count().get_result::<i64>(conn).await.unwrap(),
                        0
                    );
                }))
                .await;

                // insert data into each database
                join_all(conn_pools.iter().map(|conn_pool| async move {
                    let conn = &mut conn_pool.get().await.unwrap();
                    insert_into(book::table)
                        .values(NewBook {
                            title: "Title".into(),
                        })
                        .execute(conn)
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
                    let conn = &mut conn_pool.get().await.unwrap();
                    assert_eq!(
                        book::table.count().get_result::<i64>(conn).await.unwrap(),
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
        let backend = create_backend(false).await;
        test_pool_drops_created_restricted_databases(backend).await;
    }

    #[test(flavor = "multi_thread", shared)]
    async fn pool_drops_created_unrestricted_database() {
        let backend = create_backend(false).await;
        test_pool_drops_created_unrestricted_database(backend).await;
    }
}
