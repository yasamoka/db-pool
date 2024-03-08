use std::{borrow::Cow, pin::Pin};

use async_trait::async_trait;
use bb8::{Builder, Pool, PooledConnection};
use diesel::{prelude::*, sql_query, table};
use diesel_async::{
    pooled_connection::AsyncDieselConnectionManager, AsyncConnection, AsyncMysqlConnection,
    RunQueryDsl,
};
use futures::Future;
use uuid::Uuid;

use crate::{statement::mysql, util::get_db_name};

use super::r#trait::{impl_async_backend_for_async_mysql_backend, AsyncMySQLBackend};

type Manager = AsyncDieselConnectionManager<AsyncMysqlConnection>;

pub struct DieselAsyncMysqlBackend {
    username: String,
    password: String,
    host: String,
    port: u16,
    default_pool: Pool<Manager>,
    create_entities: Box<
        dyn Fn(AsyncMysqlConnection) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
            + Send
            + Sync
            + 'static,
    >,
    create_pool_builder: Box<dyn Fn() -> Builder<Manager> + Send + Sync + 'static>,
    drop_previous_databases_flag: bool,
}

impl DieselAsyncMysqlBackend {
    pub fn new(
        username: String,
        password: String,
        host: String,
        port: u16,
        default_pool: Pool<Manager>,
        create_entities: impl Fn(AsyncMysqlConnection) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
            + Send
            + Sync
            + 'static,
        create_pool_builder: impl Fn() -> Builder<Manager> + Send + Sync + 'static,
    ) -> Self {
        Self {
            username,
            password,
            host,
            port,
            default_pool,
            create_entities: Box::new(create_entities),
            create_pool_builder: Box::new(create_pool_builder),
            drop_previous_databases_flag: true,
        }
    }

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
impl AsyncMySQLBackend for DieselAsyncMysqlBackend {
    type ConnectionManager = Manager;

    async fn get_connection(&self) -> PooledConnection<Manager> {
        self.default_pool.get().await.unwrap()
    }

    async fn execute_stmt(&self, query: &str, conn: &mut AsyncMysqlConnection) {
        sql_query(query).execute(conn).await.unwrap();
    }

    async fn batch_execute_stmt<'a>(
        &self,
        query: impl IntoIterator<Item = Cow<'a, str>> + Send,
        conn: &mut AsyncMysqlConnection,
    ) {
        let query = query.into_iter().collect::<Vec<_>>().join(";");
        self.execute_stmt(query.as_str(), conn).await;
    }

    fn get_host(&self) -> &str {
        self.host.as_str()
    }

    async fn get_previous_database_names(&self, conn: &mut AsyncMysqlConnection) -> Vec<String> {
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
            .unwrap()
    }

    async fn create_entities(&self, db_name: &str) {
        let database_url =
            self.create_database_url(self.username.as_str(), self.password.as_str(), db_name);
        let conn = AsyncMysqlConnection::establish(database_url.as_str())
            .await
            .unwrap();
        (self.create_entities)(conn).await;
    }

    async fn create_connection_pool(&self, db_id: Uuid) -> Pool<Self::ConnectionManager> {
        let db_name = get_db_name(db_id);
        let db_name = db_name.as_str();
        let database_url = self.create_passwordless_database_url(db_name, db_name);
        let manager =
            AsyncDieselConnectionManager::<AsyncMysqlConnection>::new(database_url.as_str());
        (self.create_pool_builder)().build(manager).await.unwrap()
    }

    async fn get_table_names(&self, db_name: &str, conn: &mut AsyncMysqlConnection) -> Vec<String> {
        table! {
            tables (table_name) {
                table_name -> Text,
                table_schema -> Text
            }
        }

        sql_query(mysql::USE_DEFAULT_DATABASE)
            .execute(conn)
            .await
            .unwrap();

        tables::table
            .filter(tables::table_schema.eq(db_name))
            .select(tables::table_name)
            .load::<String>(conn)
            .await
            .unwrap()
    }

    fn get_drop_previous_databases(&self) -> bool {
        self.drop_previous_databases_flag
    }
}

impl_async_backend_for_async_mysql_backend!(DieselAsyncMysqlBackend, Manager);
