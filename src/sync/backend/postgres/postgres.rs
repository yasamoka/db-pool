use std::{borrow::Cow, collections::HashMap, ops::Deref};

use parking_lot::Mutex;
use r2d2::{Builder, Pool, PooledConnection};
use r2d2_postgres::{
    postgres::{Client, Config, Error, NoTls},
    PostgresConnectionManager,
};
use uuid::Uuid;

use crate::{common::statement::postgres, util::get_db_name};

use super::{
    super::error::Error as BackendError,
    r#trait::{impl_backend_for_pg_backend, PostgresBackend as PostgresBackendTrait},
};

type Manager = PostgresConnectionManager<NoTls>;

pub struct PostgresBackend {
    config: Config,
    default_pool: Pool<Manager>,
    db_conns: Mutex<HashMap<Uuid, Client>>,
    create_restricted_pool: Box<dyn Fn() -> Builder<Manager> + Send + Sync + 'static>,
    create_entities: Box<dyn Fn(&mut Client) + Send + Sync + 'static>,
    drop_previous_databases_flag: bool,
}

impl PostgresBackend {
    pub fn new(
        config: Config,
        create_default_pool: impl Fn() -> Builder<Manager>,
        create_restricted_pool: impl Fn() -> Builder<Manager> + Send + Sync + 'static,
        create_entities: impl Fn(&mut Client) + Send + Sync + 'static,
    ) -> Result<Self, r2d2::Error> {
        let manager = Manager::new(config.clone(), NoTls);
        let default_pool = (create_default_pool()).build(manager)?;

        Ok(Self {
            config,
            default_pool,
            db_conns: Mutex::new(HashMap::new()),
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
}

impl PostgresBackendTrait for PostgresBackend {
    type ConnectionManager = Manager;
    type ConnectionError = ConnectionError;
    type QueryError = QueryError;

    fn execute(&self, query: &str, conn: &mut Client) -> Result<(), QueryError> {
        conn.execute(query, &[])?;
        Ok(())
    }

    fn batch_execute<'a>(
        &self,
        query: impl IntoIterator<Item = Cow<'a, str>>,
        conn: &mut Client,
    ) -> Result<(), QueryError> {
        let query = query.into_iter().collect::<Vec<_>>().join(";");
        conn.batch_execute(query.as_str())?;
        Ok(())
    }

    fn get_default_connection(&self) -> Result<PooledConnection<Manager>, r2d2::Error> {
        self.default_pool.get()
    }

    fn establish_database_connection(&self, db_id: Uuid) -> Result<Client, ConnectionError> {
        let mut config = self.config.clone();
        let db_name = get_db_name(db_id);
        config.dbname(db_name.as_str());
        config.connect(NoTls).map_err(Into::into)
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

    fn get_previous_database_names(&self, conn: &mut Client) -> Result<Vec<String>, QueryError> {
        conn.query(postgres::GET_DATABASE_NAMES, &[])
            .map(|rows| rows.iter().map(|row| row.get(0)).collect())
            .map_err(Into::into)
    }

    fn create_entities(&self, conn: &mut Client) {
        (self.create_entities)(conn);
    }

    fn create_connection_pool(&self, db_id: Uuid) -> Result<Pool<Manager>, r2d2::Error> {
        let mut config = self.config.clone();
        let db_name = get_db_name(db_id);
        let db_name = db_name.as_str();
        config.dbname(db_name);
        config.user(db_name);
        let manager = PostgresConnectionManager::new(config, NoTls);
        (self.create_restricted_pool)().build(manager)
    }

    fn get_table_names(&self, conn: &mut Client) -> Result<Vec<String>, QueryError> {
        conn.query(postgres::GET_TABLE_NAMES, &[])
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

impl From<ConnectionError> for BackendError<ConnectionError, QueryError> {
    fn from(value: ConnectionError) -> Self {
        Self::Connection(value)
    }
}

impl From<QueryError> for BackendError<ConnectionError, QueryError> {
    fn from(value: QueryError) -> Self {
        Self::Query(value)
    }
}

impl_backend_for_pg_backend!(PostgresBackend, Manager, ConnectionError, QueryError);
