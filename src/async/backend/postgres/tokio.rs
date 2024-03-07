use std::{borrow::Cow, collections::HashMap, pin::Pin};

use async_trait::async_trait;
use bb8::{Builder, Pool};
use bb8_postgres::{
    tokio_postgres::{Client, Config, NoTls},
    PostgresConnectionManager,
};
use futures::Future;
use parking_lot::Mutex;
use uuid::Uuid;

use crate::{statement::pg, util::get_db_name};

use super::r#trait::{impl_async_backend_for_async_pg_backend, AsyncPgBackend};

type Manager = PostgresConnectionManager<NoTls>;

pub struct TokioPostgresBackend {
    privileged_config: Config,
    default_pool: Pool<Manager>,
    db_conns: Mutex<HashMap<Uuid, Client>>,
    create_entities: Box<
        dyn Fn(Client) -> Pin<Box<dyn Future<Output = Client> + Send + 'static>>
            + Send
            + Sync
            + 'static,
    >,
    create_pool_builder: Box<dyn Fn() -> Builder<Manager> + Send + Sync + 'static>,
    terminate_connections_before_drop: bool,
}

impl TokioPostgresBackend {
    pub fn new(
        privileged_config: Config,
        default_pool: Pool<Manager>,
        create_entities: impl Fn(Client) -> Pin<Box<dyn Future<Output = Client> + Send + 'static>>
            + Send
            + Sync
            + 'static,
        create_pool_builder: impl Fn() -> Builder<Manager> + Send + Sync + 'static,
        terminate_connections_before_drop: bool,
    ) -> Self {
        Self {
            privileged_config,
            default_pool,
            db_conns: Mutex::new(HashMap::new()),
            create_entities: Box::new(create_entities),
            create_pool_builder: Box::new(create_pool_builder),
            terminate_connections_before_drop,
        }
    }
}

#[async_trait]
impl AsyncPgBackend for TokioPostgresBackend {
    type ConnectionManager = Manager;

    async fn execute_stmt(&self, query: &str, conn: &mut Client) {
        conn.execute(query, &[]).await.unwrap();
    }

    async fn batch_execute_stmt<'a>(
        &self,
        query: impl IntoIterator<Item = Cow<'a, str>> + Send,
        conn: &mut Client,
    ) {
        let query = query.into_iter().collect::<Vec<_>>().join(";");
        conn.batch_execute(query.as_str()).await.unwrap();
    }

    async fn get_default_connection(&self) -> bb8::PooledConnection<Self::ConnectionManager> {
        self.default_pool.get().await.unwrap()
    }

    async fn establish_database_connection(&self, db_id: Uuid) -> Client {
        let mut config = self.privileged_config.clone();
        let db_name = get_db_name(db_id);
        config.dbname(db_name.as_str());
        let (client, connection) = config.connect(NoTls).await.unwrap();
        tokio::spawn(async move { connection.await });
        client
    }

    fn put_database_connection(&self, db_id: Uuid, conn: Client) {
        self.db_conns.lock().insert(db_id, conn);
    }

    fn get_database_connection(&self, db_id: Uuid) -> Client {
        self.db_conns.lock().remove(&db_id).unwrap()
    }

    async fn create_entities(&self, conn: Client) -> Client {
        (self.create_entities)(conn).await
    }

    async fn create_connection_pool(&self, db_id: Uuid) -> Pool<Self::ConnectionManager> {
        let mut config = self.privileged_config.clone();
        let db_name = get_db_name(db_id);
        let db_name = db_name.as_str();
        config.dbname(db_name);
        config.user(db_name);
        let manager = PostgresConnectionManager::new(config, NoTls);
        (self.create_pool_builder)().build(manager).await.unwrap()
    }

    async fn get_table_names(&self, privileged_conn: &mut Client) -> Vec<String> {
        privileged_conn
            .query(pg::GET_TABLE_NAMES, &[])
            .await
            .unwrap()
            .drain(..)
            .map(|row| row.get(0))
            .collect()
    }

    fn terminate_connections(&self) -> bool {
        self.terminate_connections_before_drop
    }
}

impl_async_backend_for_async_pg_backend!(TokioPostgresBackend, Manager);
