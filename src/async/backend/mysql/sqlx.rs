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

type CreateEntities = dyn Fn(MySqlConnection) -> Pin<Box<dyn Future<Output = MySqlConnection> + Send + 'static>>
    + Send
    + Sync
    + 'static;

pub struct SqlxMySQLBackend {
    privileged_opts: MySqlConnectOptions,
    default_pool: MySqlPool,
    create_restricted_pool: Box<dyn Fn() -> MySqlPoolOptions + Send + Sync + 'static>,
    create_entities: Box<CreateEntities>,
    drop_previous_databases_flag: bool,
}

impl SqlxMySQLBackend {
    pub fn new(
        privileged_options: MySqlConnectOptions,
        create_privileged_pool: impl Fn() -> MySqlPoolOptions,
        create_restricted_pool: impl Fn() -> MySqlPoolOptions + Send + Sync + 'static,
        create_entities: impl Fn(MySqlConnection) -> Pin<Box<dyn Future<Output = MySqlConnection> + Send + 'static>>
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

    async fn execute_stmt(
        &self,
        query: &str,
        conn: &mut MySqlConnection,
    ) -> Result<(), QueryError> {
        conn.execute(query).await?;
        Ok(())
    }

    async fn batch_execute_stmt<'a>(
        &self,
        query: impl IntoIterator<Item = Cow<'a, str>> + Send,
        conn: &mut MySqlConnection,
    ) -> Result<(), QueryError> {
        let chunks = query.into_iter().collect::<Vec<_>>();
        if chunks.is_empty() {
            Ok(())
        } else {
            let query = chunks.join(";");
            self.execute_stmt(query.as_str(), conn).await
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

    async fn create(&self, db_id: uuid::Uuid) -> Result<MySqlPool, BError> {
        MySQLBackendWrapper::new(self).create(db_id).await
    }

    async fn clean(&self, db_id: uuid::Uuid) -> Result<(), BError> {
        MySQLBackendWrapper::new(self).clean(db_id).await
    }

    async fn drop(&self, db_id: uuid::Uuid) -> Result<(), BError> {
        MySQLBackendWrapper::new(self).drop(db_id).await
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::needless_return)]

    use sqlx::{
        mysql::{MySqlConnectOptions, MySqlPoolOptions},
        Executor,
    };
    use tokio_shared_rt::test;

    use crate::common::statement::mysql::tests::CREATE_ENTITIES_STATEMENT;

    use super::{
        super::r#trait::tests::{
            test_cleans_database_with_tables, test_cleans_database_without_tables,
            test_creates_database_with_restricted_privileges, test_drops_database,
            test_drops_previous_databases,
        },
        SqlxMySQLBackend,
    };

    fn create_backend(with_table: bool) -> SqlxMySQLBackend {
        SqlxMySQLBackend::new(
            MySqlConnectOptions::new().username("root").password("root"),
            MySqlPoolOptions::new,
            MySqlPoolOptions::new,
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
    async fn drops_previous_databases() {
        test_drops_previous_databases(
            create_backend(false),
            create_backend(false).drop_previous_databases(true),
            create_backend(false).drop_previous_databases(false),
        )
        .await;
    }

    #[test(flavor = "multi_thread", shared)]
    async fn creates_database_with_restricted_privileges() {
        let backend = create_backend(true).drop_previous_databases(false);
        test_creates_database_with_restricted_privileges(backend).await;
    }

    #[test(flavor = "multi_thread", shared)]
    async fn cleans_database_with_tables() {
        let backend = create_backend(true).drop_previous_databases(false);
        test_cleans_database_with_tables(backend).await;
    }

    #[test(flavor = "multi_thread", shared)]
    async fn cleans_database_without_tables() {
        let backend = create_backend(false).drop_previous_databases(false);
        test_cleans_database_without_tables(backend).await;
    }

    #[test(flavor = "multi_thread", shared)]
    async fn drops_database() {
        let backend = create_backend(true).drop_previous_databases(false);
        test_drops_database(backend).await;
    }
}
