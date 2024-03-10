use std::{borrow::Cow, collections::HashMap, convert::Into, ops::Deref, pin::Pin};

use async_trait::async_trait;
use bb8::{Builder, Pool, RunError};
use bb8_postgres::{
    tokio_postgres::{Client, Config, Error, NoTls},
    PostgresConnectionManager,
};
use futures::Future;
use parking_lot::Mutex;
use uuid::Uuid;

use crate::{common::statement::postgres, util::get_db_name};

use super::{
    super::error::Error as BackendError,
    r#trait::{impl_async_backend_for_async_pg_backend, AsyncPgBackend},
};

type Manager = PostgresConnectionManager<NoTls>;
type CreateEntities = dyn Fn(Client) -> Pin<Box<dyn Future<Output = Client> + Send + 'static>>
    + Send
    + Sync
    + 'static;

pub struct Backend {
    config: Config,
    default_pool: Pool<Manager>,
    db_conns: Mutex<HashMap<Uuid, Client>>,
    create_restricted_pool: Box<dyn Fn() -> Builder<Manager> + Send + Sync + 'static>,
    create_entities: Box<CreateEntities>,
    drop_previous_databases_flag: bool,
}

impl Backend {
    pub async fn new(
        config: Config,
        create_privileged_pool: impl Fn() -> Builder<Manager>,
        create_restricted_pool: impl Fn() -> Builder<Manager> + Send + Sync + 'static,
        create_entities: impl Fn(Client) -> Pin<Box<dyn Future<Output = Client> + Send + 'static>>
            + Send
            + Sync
            + 'static,
    ) -> Result<Self, RunError<Error>> {
        let manager = Manager::new(config.clone(), NoTls);
        let default_pool = (create_privileged_pool()).build(manager).await?;

        Ok(Self {
            config,
            default_pool,
            db_conns: Mutex::new(HashMap::new()),
            create_entities: Box::new(create_entities),
            create_restricted_pool: Box::new(create_restricted_pool),
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
}

#[async_trait]
impl AsyncPgBackend for Backend {
    type ConnectionManager = Manager;
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
        &self,
    ) -> Result<bb8::PooledConnection<Self::ConnectionManager>, RunError<Error>> {
        self.default_pool.get().await
    }

    async fn establish_database_connection(&self, db_id: Uuid) -> Result<Client, ConnectionError> {
        let mut config = self.config.clone();
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

    async fn create_connection_pool(
        &self,
        db_id: Uuid,
    ) -> Result<Pool<Self::ConnectionManager>, RunError<Error>> {
        let mut config = self.config.clone();
        let db_name = get_db_name(db_id);
        let db_name = db_name.as_str();
        config.dbname(db_name);
        config.user(db_name);
        let manager = PostgresConnectionManager::new(config, NoTls);
        (self.create_restricted_pool)()
            .build(manager)
            .await
            .map_err(Into::into)
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

#[derive(Debug)]
pub struct ConnectionError(Error);

impl Deref for ConnectionError {
    type Target = Error;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Error> for ConnectionError {
    fn from(value: Error) -> Self {
        Self(value)
    }
}

#[derive(Debug)]
pub struct QueryError(Error);

impl Deref for QueryError {
    type Target = Error;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Error> for QueryError {
    fn from(value: Error) -> Self {
        Self(value)
    }
}

impl From<ConnectionError> for BackendError<Error, ConnectionError, QueryError> {
    fn from(value: ConnectionError) -> Self {
        Self::Connection(value)
    }
}

impl From<QueryError> for BackendError<Error, ConnectionError, QueryError> {
    fn from(value: QueryError) -> Self {
        Self::Query(value)
    }
}

impl_async_backend_for_async_pg_backend!(Backend, Manager, ConnectionError, QueryError);
