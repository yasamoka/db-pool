use std::{borrow::Cow, pin::Pin};

use async_trait::async_trait;
use diesel::{prelude::*, result::Error, sql_query, table};
use diesel_async::{
    pooled_connection::AsyncDieselConnectionManager, AsyncConnection, AsyncMysqlConnection,
    RunQueryDsl,
};
use futures::Future;
use uuid::Uuid;

use crate::{
    common::{config::mysql::PrivilegedConfig, statement::mysql},
    util::get_db_name,
};

use super::{
    super::{
        common::pool::diesel::r#trait::DieselPoolAssociation, error::Error as BackendError,
        r#trait::Backend,
    },
    r#trait::{MySQLBackend, MySQLBackendWrapper},
};

type CreateEntities = dyn Fn(AsyncMysqlConnection) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
    + Send
    + Sync
    + 'static;

pub struct DieselAsyncMySQLBackend<P>
where
    P: DieselPoolAssociation<AsyncMysqlConnection>,
{
    privileged_config: PrivilegedConfig,
    default_pool: P::Pool,
    create_restricted_pool: Box<dyn Fn() -> P::Builder + Send + Sync + 'static>,
    create_entities: Box<CreateEntities>,
    drop_previous_databases_flag: bool,
}

impl<P> DieselAsyncMySQLBackend<P>
where
    P: DieselPoolAssociation<AsyncMysqlConnection>,
{
    pub async fn new(
        privileged_config: PrivilegedConfig,
        create_privileged_pool: impl Fn() -> P::Builder,
        create_restricted_pool: impl Fn() -> P::Builder + Send + Sync + 'static,
        create_entities: impl Fn(AsyncMysqlConnection) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
            + Send
            + Sync
            + 'static,
    ) -> Result<Self, P::BuildError> {
        let manager = AsyncDieselConnectionManager::new(privileged_config.default_connection_url());
        let builder = create_privileged_pool();
        let default_pool = P::build_pool(builder, manager).await?;

        Ok(Self {
            privileged_config,
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
}

#[async_trait]
impl<'pool, P> MySQLBackend<'pool> for DieselAsyncMySQLBackend<P>
where
    P: DieselPoolAssociation<AsyncMysqlConnection>,
{
    type Connection = AsyncMysqlConnection;
    type PooledConnection = P::PooledConnection<'pool>;
    type Pool = P::Pool;

    type BuildError = P::BuildError;
    type PoolError = P::PoolError;
    type ConnectionError = ConnectionError;
    type QueryError = Error;

    async fn get_connection(&'pool self) -> Result<P::PooledConnection<'pool>, P::PoolError> {
        P::get_connection(&self.default_pool).await
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
        self.privileged_config.host.as_str()
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
        let database_url = self
            .privileged_config
            .privileged_database_connection_url(db_name);
        let conn = AsyncMysqlConnection::establish(database_url.as_str()).await?;
        (self.create_entities)(conn).await;
        Ok(())
    }

    async fn create_connection_pool(&self, db_id: Uuid) -> Result<P::Pool, P::BuildError> {
        let db_name = get_db_name(db_id);
        let db_name = db_name.as_str();
        let database_url = self.privileged_config.restricted_database_connection_url(
            db_name,
            Some(db_name),
            db_name,
        );
        let manager =
            AsyncDieselConnectionManager::<AsyncMysqlConnection>::new(database_url.as_str());
        let builder = (self.create_restricted_pool)();
        P::build_pool(builder, manager).await
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

type BError<BuildError, PoolError> = BackendError<BuildError, PoolError, ConnectionError, Error>;

#[async_trait]
impl<P> Backend for DieselAsyncMySQLBackend<P>
where
    P: DieselPoolAssociation<AsyncMysqlConnection>,
{
    type Pool = P::Pool;

    type BuildError = P::BuildError;
    type PoolError = P::PoolError;
    type ConnectionError = ConnectionError;
    type QueryError = Error;

    async fn init(&self) -> Result<(), BError<P::BuildError, P::PoolError>> {
        MySQLBackendWrapper::new(self).init().await
    }

    async fn create(
        &self,
        db_id: uuid::Uuid,
    ) -> Result<P::Pool, BError<P::BuildError, P::PoolError>> {
        MySQLBackendWrapper::new(self).create(db_id).await
    }

    async fn clean(&self, db_id: uuid::Uuid) -> Result<(), BError<P::BuildError, P::PoolError>> {
        MySQLBackendWrapper::new(self).clean(db_id).await
    }

    async fn drop(&self, db_id: uuid::Uuid) -> Result<(), BError<P::BuildError, P::PoolError>> {
        MySQLBackendWrapper::new(self).drop(db_id).await
    }
}
