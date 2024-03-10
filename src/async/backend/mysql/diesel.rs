use std::{borrow::Cow, convert::Into, pin::Pin};

use async_trait::async_trait;
use bb8::{Builder, Pool, PooledConnection, RunError};
use diesel::{prelude::*, result::Error, sql_query, table};
use diesel_async::{
    pooled_connection::{AsyncDieselConnectionManager, PoolError},
    AsyncConnection, AsyncMysqlConnection, RunQueryDsl,
};
use futures::Future;
use uuid::Uuid;

use crate::{common::statement::mysql, util::get_db_name};

use super::r#trait::{impl_async_backend_for_async_mysql_backend, AsyncMySQLBackend};

type Manager = AsyncDieselConnectionManager<AsyncMysqlConnection>;
type CreateEntities = dyn Fn(AsyncMysqlConnection) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
    + Send
    + Sync
    + 'static;

pub struct Backend {
    username: String,
    password: String,
    host: String,
    port: u16,
    default_pool: Pool<Manager>,
    create_restricted_pool: Box<dyn Fn() -> Builder<Manager> + Send + Sync + 'static>,
    create_entities: Box<CreateEntities>,
    drop_previous_databases_flag: bool,
}

impl Backend {
    pub async fn new(
        username: String,
        password: String,
        host: String,
        port: u16,
        create_privileged_pool: impl Fn() -> Builder<Manager>,
        create_restricted_pool: impl Fn() -> Builder<Manager> + Send + Sync + 'static,
        create_entities: impl Fn(AsyncMysqlConnection) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
            + Send
            + Sync
            + 'static,
    ) -> Result<Self, PoolError> {
        let connection_url = format!("mysql://{username}:{password}@{host}:{port}");
        let manager = AsyncDieselConnectionManager::new(connection_url);
        let default_pool = (create_privileged_pool()).build(manager).await?;

        Ok(Self {
            username,
            password,
            host,
            port,
            default_pool,
            create_restricted_pool: Box::new(create_restricted_pool),
            create_entities: Box::new(create_entities),
            drop_previous_databases_flag: true,
        })
    }

    #[must_use]
    pub fn drop_previous_databases(self, value: bool) -> Self {
        Self {
            drop_previous_databases_flag: value,
            ..self
        }
    }

    fn create_database_url(&self, username: &str, password: &str, db_name: &str) -> String {
        format!(
            "mysql://{}:{}@{}:{}/{}",
            username, password, self.host, self.port, db_name
        )
    }

    fn create_passwordless_database_url(&self, username: &str, db_name: &str) -> String {
        format!(
            "mysql://{}@{}:{}/{}",
            username, self.host, self.port, db_name
        )
    }
}

#[async_trait]
impl AsyncMySQLBackend for Backend {
    type ConnectionManager = Manager;
    type ConnectionError = ConnectionError;
    type QueryError = Error;

    async fn get_connection(&self) -> Result<PooledConnection<Manager>, RunError<PoolError>> {
        self.default_pool.get().await
    }

    async fn execute_stmt(&self, query: &str, conn: &mut AsyncMysqlConnection) -> QueryResult<()> {
        sql_query(query).execute(conn).await?;
        Ok(())
    }

    async fn batch_execute_stmt<'a>(
        &self,
        query: impl IntoIterator<Item = Cow<'a, str>> + Send,
        conn: &mut AsyncMysqlConnection,
    ) -> QueryResult<()> {
        let query = query.into_iter().collect::<Vec<_>>().join(";");
        self.execute_stmt(query.as_str(), conn).await
    }

    fn get_host(&self) -> &str {
        self.host.as_str()
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
        let database_url =
            self.create_database_url(self.username.as_str(), self.password.as_str(), db_name);
        let conn = AsyncMysqlConnection::establish(database_url.as_str()).await?;
        (self.create_entities)(conn).await;
        Ok(())
    }

    async fn create_connection_pool(
        &self,
        db_id: Uuid,
    ) -> Result<Pool<Self::ConnectionManager>, RunError<PoolError>> {
        let db_name = get_db_name(db_id);
        let db_name = db_name.as_str();
        let database_url = self.create_passwordless_database_url(db_name, db_name);
        let manager =
            AsyncDieselConnectionManager::<AsyncMysqlConnection>::new(database_url.as_str());
        (self.create_restricted_pool)()
            .build(manager)
            .await
            .map_err(Into::into)
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

impl_async_backend_for_async_mysql_backend!(Backend, Manager, ConnectionError, Error);
