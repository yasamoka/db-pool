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