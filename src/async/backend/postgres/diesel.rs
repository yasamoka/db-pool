use std::{borrow::Cow, collections::HashMap, convert::Into, pin::Pin};

use async_trait::async_trait;
use bb8::{Builder, Pool, PooledConnection, RunError};
use diesel::{prelude::*, result::Error, sql_query, table, ConnectionError};
use diesel_async::{
    pooled_connection::{AsyncDieselConnectionManager, PoolError},
    AsyncConnection as _, AsyncPgConnection, RunQueryDsl,
};
use futures::Future;
use parking_lot::Mutex;
use uuid::Uuid;

use crate::{common::config::postgres::PrivilegedConfig, util::get_db_name};

use super::r#trait::{impl_async_backend_for_async_pg_backend, AsyncPgBackend};

type Manager = AsyncDieselConnectionManager<AsyncPgConnection>;
type CreateEntities = dyn Fn(AsyncPgConnection) -> Pin<Box<dyn Future<Output = AsyncPgConnection> + Send + 'static>>
    + Send
    + Sync
    + 'static;

pub struct DieselAsyncPostgresBackend {
    privileged_config: PrivilegedConfig,
    default_pool: Pool<Manager>,
    db_conns: Mutex<HashMap<Uuid, AsyncPgConnection>>,
    create_restricted_pool: Box<dyn Fn() -> Builder<Manager> + Send + Sync + 'static>,
    create_entities: Box<CreateEntities>,
    drop_previous_databases_flag: bool,
}

impl DieselAsyncPostgresBackend {
    pub async fn new(
        privileged_config: PrivilegedConfig,
        create_privileged_pool: impl Fn() -> Builder<Manager>,
        create_restricted_pool: impl Fn() -> Builder<Manager> + Send + Sync + 'static,
        create_entities: impl Fn(
                AsyncPgConnection,
            ) -> Pin<Box<dyn Future<Output = AsyncPgConnection> + Send + 'static>>
            + Send
            + Sync
            + 'static,
    ) -> Result<Self, PoolError> {
        let manager = AsyncDieselConnectionManager::new(privileged_config.default_connection_url());
        let default_pool = (create_privileged_pool()).build(manager).await?;

        Ok(Self {
            privileged_config,
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

#[async_trait]
impl AsyncPgBackend for DieselAsyncPostgresBackend {
    type ConnectionManager = Manager;
    type ConnectionError = ConnectionError;
    type QueryError = Error;

    async fn execute_stmt(&self, query: &str, conn: &mut AsyncPgConnection) -> QueryResult<()> {
        sql_query(query).execute(conn).await?;
        Ok(())
    }

    async fn batch_execute_stmt<'a>(
        &self,
        query: impl IntoIterator<Item = Cow<'a, str>> + Send,
        conn: &mut AsyncPgConnection,
    ) -> QueryResult<()> {
        let query = query.into_iter().collect::<Vec<_>>().join(";");
        self.execute_stmt(query.as_str(), conn).await
    }

    async fn get_default_connection(
        &self,
    ) -> Result<PooledConnection<Manager>, RunError<PoolError>> {
        self.default_pool.get().await
    }

    async fn establish_database_connection(
        &self,
        db_id: Uuid,
    ) -> ConnectionResult<AsyncPgConnection> {
        let db_name = get_db_name(db_id);
        let database_url = self
            .privileged_config
            .privileged_database_connection_url(db_name.as_str());
        AsyncPgConnection::establish(database_url.as_str()).await
    }

    fn put_database_connection(&self, db_id: Uuid, conn: AsyncPgConnection) {
        self.db_conns.lock().insert(db_id, conn);
    }

    fn get_database_connection(&self, db_id: Uuid) -> AsyncPgConnection {
        self.db_conns
            .lock()
            .remove(&db_id)
            .unwrap_or_else(|| panic!("connection map must have a connection for {db_id}"))
    }

    async fn get_previous_database_names(
        &self,
        conn: &mut AsyncPgConnection,
    ) -> QueryResult<Vec<String>> {
        table! {
            pg_database (oid) {
                oid -> Int4,
                datname -> Text
            }
        }

        pg_database::table
            .select(pg_database::datname)
            .filter(pg_database::datname.like("db_pool_%"))
            .load::<String>(conn)
            .await
    }

    async fn create_entities(&self, conn: AsyncPgConnection) -> AsyncPgConnection {
        (self.create_entities)(conn).await
    }

    async fn create_connection_pool(
        &self,
        db_id: Uuid,
    ) -> Result<Pool<Manager>, RunError<PoolError>> {
        let db_name = get_db_name(db_id);
        let db_name = db_name.as_str();
        let database_url = self.privileged_config.restricted_database_connection_url(
            db_name,
            Some(db_name),
            db_name,
        );
        let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url.as_str());
        (self.create_restricted_pool)()
            .build(manager)
            .await
            .map_err(Into::into)
    }

    async fn get_table_names(
        &self,
        privileged_conn: &mut AsyncPgConnection,
    ) -> QueryResult<Vec<String>> {
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
    }

    fn get_drop_previous_databases(&self) -> bool {
        self.drop_previous_databases_flag
    }
}

impl_async_backend_for_async_pg_backend!(
    DieselAsyncPostgresBackend,
    Manager,
    ConnectionError,
    Error
);
