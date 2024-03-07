use std::{borrow::Cow, collections::HashMap, pin::Pin};

use async_trait::async_trait;
use bb8::{Builder, Pool};
use diesel::{prelude::*, sql_query, table};
use diesel_async::{
    pooled_connection::AsyncDieselConnectionManager, AsyncConnection as _, AsyncPgConnection,
    RunQueryDsl,
};
use futures::Future;
use parking_lot::Mutex;
use uuid::Uuid;

use crate::util::get_db_name;

use super::r#trait::{impl_async_backend_for_async_pg_backend, AsyncPgBackend};

type Manager = AsyncDieselConnectionManager<AsyncPgConnection>;

pub struct DieselAsyncPgBackend<CE, CPB>
where
    CE: Fn(AsyncPgConnection) -> Pin<Box<dyn Future<Output = AsyncPgConnection> + Send + 'static>>
        + Send
        + Sync
        + 'static,
    CPB: Fn() -> Builder<Manager> + Send + Sync + 'static,
{
    username: String,
    password: String,
    host: String,
    port: u16,
    default_pool: Pool<Manager>,
    db_conns: Mutex<HashMap<Uuid, AsyncPgConnection>>,
    create_entities: CE,
    create_pool_builder: CPB,
    terminate_connections_before_drop: bool,
}

impl<CE, CPB> DieselAsyncPgBackend<CE, CPB>
where
    CE: Fn(AsyncPgConnection) -> Pin<Box<dyn Future<Output = AsyncPgConnection> + Send + 'static>>
        + Send
        + Sync
        + 'static,
    CPB: Fn() -> Builder<Manager> + Send + Sync + 'static,
{
    pub fn new(
        username: String,
        password: String,
        host: String,
        port: u16,
        default_pool: Pool<Manager>,
        create_entities: CE,
        create_pool_builder: CPB,
        terminate_connections_before_drop: bool,
    ) -> Self {
        Self {
            username,
            password,
            host,
            port,
            default_pool,
            db_conns: Mutex::new(HashMap::new()),
            create_entities,
            create_pool_builder,
            terminate_connections_before_drop,
        }
    }

    fn create_database_url(&self, username: &str, password: &str, db_name: &str) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            username, password, self.host, self.port, db_name
        )
    }
}

#[async_trait]
impl<CE, CPB> AsyncPgBackend for DieselAsyncPgBackend<CE, CPB>
where
    CE: Fn(AsyncPgConnection) -> Pin<Box<dyn Future<Output = AsyncPgConnection> + Send + 'static>>
        + Send
        + Sync
        + 'static,
    CPB: Fn() -> Builder<Manager> + Send + Sync + 'static,
{
    type ConnectionManager = Manager;

    async fn execute_stmt(&self, query: &str, conn: &mut AsyncPgConnection) {
        sql_query(query).execute(conn).await.unwrap();
    }

    async fn batch_execute_stmt<'a>(
        &self,
        query: impl IntoIterator<Item = Cow<'a, str>> + Send,
        conn: &mut AsyncPgConnection,
    ) {
        let query = query.into_iter().collect::<Vec<_>>().join(";");
        self.execute_stmt(query.as_str(), conn).await;
    }

    async fn get_default_connection(&self) -> bb8::PooledConnection<Self::ConnectionManager> {
        self.default_pool.get().await.unwrap()
    }

    async fn establish_database_connection(&self, db_id: Uuid) -> AsyncPgConnection {
        let db_name = get_db_name(db_id);
        let database_url = self.create_database_url(
            self.username.as_str(),
            self.password.as_str(),
            db_name.as_str(),
        );
        AsyncPgConnection::establish(database_url.as_str())
            .await
            .unwrap()
    }

    fn put_database_connection(&self, db_id: Uuid, conn: AsyncPgConnection) {
        self.db_conns.lock().insert(db_id, conn);
    }

    fn get_database_connection(&self, db_id: Uuid) -> AsyncPgConnection {
        self.db_conns.lock().remove(&db_id).unwrap()
    }

    async fn create_entities(&self, conn: AsyncPgConnection) -> AsyncPgConnection {
        (self.create_entities)(conn).await
    }

    async fn create_connection_pool(&self, db_id: Uuid) -> Pool<Self::ConnectionManager> {
        let db_name = get_db_name(db_id);
        let db_name = db_name.as_str();
        let database_url = self.create_database_url(db_name, db_name, db_name);
        let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url.as_str());
        (self.create_pool_builder)().build(manager).await.unwrap()
    }

    async fn get_table_names(&self, privileged_conn: &mut AsyncPgConnection) -> Vec<String> {
        table! {
            pg_tables (tablename) {
                #[sql_name = "schemaname"]
                schema_name -> Text,
                tablename -> Text
            }
        }

        pg_tables::table
            .filter(pg_tables::schema_name.ne_all(["pg_catalog", "information_schema"]))
            .select(pg_tables::tablename)
            .load(privileged_conn)
            .await
            .unwrap()
    }

    fn terminate_connections(&self) -> bool {
        self.terminate_connections_before_drop
    }
}

impl_async_backend_for_async_pg_backend!(DieselAsyncPgBackend, Manager);
