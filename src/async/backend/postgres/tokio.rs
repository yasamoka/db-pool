use std::{borrow::Cow, collections::HashMap, convert::Into, pin::Pin};

use async_trait::async_trait;
use bb8::{Builder, Pool, RunError};
use bb8_postgres::{
    tokio_postgres::{Client, Config, Error, NoTls},
    PostgresConnectionManager,
};
use futures::Future;
use parking_lot::Mutex;
use uuid::Uuid;

use crate::{statement::pg, util::get_db_name};

use super::{
    super::error::Error as BackendError,
    r#trait::{impl_async_backend_for_async_pg_backend, AsyncPgBackend},
};

type Manager = PostgresConnectionManager<NoTls>;
type CreateEntities = dyn Fn(Client) -> Pin<Box<dyn Future<Output = Client> + Send + 'static>>
    + Send
    + Sync
    + 'static;

pub struct TokioPostgresBackend {
    privileged_config: Config,
    default_pool: Pool<Manager>,
    db_conns: Mutex<HashMap<Uuid, Client>>,
    create_entities: Box<CreateEntities>,
    create_pool_builder: Box<dyn Fn() -> Builder<Manager> + Send + Sync + 'static>,
    drop_previous_databases_flag: bool,
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
    ) -> Self {
        Self {
            privileged_config,
            default_pool,
            db_conns: Mutex::new(HashMap::new()),
            create_entities: Box::new(create_entities),
            create_pool_builder: Box::new(create_pool_builder),
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
impl AsyncPgBackend for TokioPostgresBackend {
    type ConnectionManager = Manager;
    type ConnectionError = Error;
    type QueryError = Error;

    async fn execute_stmt(&self, query: &str, conn: &mut Client) -> Result<(), Error> {
        conn.execute(query, &[]).await?;
        Ok(())
    }

    async fn batch_execute_stmt<'a>(
        &self,
        query: impl IntoIterator<Item = Cow<'a, str>> + Send,
        conn: &mut Client,
    ) -> Result<(), Error> {
        let query = query.into_iter().collect::<Vec<_>>().join(";");
        conn.batch_execute(query.as_str()).await?;
        Ok(())
    }

    async fn get_default_connection(
        &self,
    ) -> Result<bb8::PooledConnection<Self::ConnectionManager>, RunError<Error>> {
        self.default_pool.get().await
    }

    async fn establish_database_connection(&self, db_id: Uuid) -> Result<Client, Error> {
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

    async fn get_previous_database_names(&self, conn: &mut Client) -> Result<Vec<String>, Error> {
        conn.query(pg::GET_DATABASE_NAMES, &[])
            .await
            .map(|rows| rows.iter().map(|row| row.get(0)).collect())
    }

    async fn create_entities(&self, conn: Client) -> Client {
        (self.create_entities)(conn).await
    }

    async fn create_connection_pool(
        &self,
        db_id: Uuid,
    ) -> Result<Pool<Self::ConnectionManager>, RunError<Error>> {
        let mut config = self.privileged_config.clone();
        let db_name = get_db_name(db_id);
        let db_name = db_name.as_str();
        config.dbname(db_name);
        config.user(db_name);
        let manager = PostgresConnectionManager::new(config, NoTls);
        (self.create_pool_builder)()
            .build(manager)
            .await
            .map_err(Into::into)
    }

    async fn get_table_names(&self, privileged_conn: &mut Client) -> Result<Vec<String>, Error> {
        privileged_conn
            .query(pg::GET_TABLE_NAMES, &[])
            .await
            .map(|rows| rows.iter().map(|row| row.get(0)).collect())
    }

    fn get_drop_previous_databases(&self) -> bool {
        self.drop_previous_databases_flag
    }
}

// TODO: separate connection error and query error

impl From<Error> for BackendError<Error, Error, Error> {
    fn from(value: Error) -> Self {
        Self::Query(value)
    }
}

impl_async_backend_for_async_pg_backend!(TokioPostgresBackend, Manager, Error, Error);
