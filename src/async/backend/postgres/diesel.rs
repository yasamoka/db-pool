use std::{borrow::Cow, collections::HashMap, pin::Pin};

use async_trait::async_trait;
use diesel::{prelude::*, result::Error, sql_query, table, ConnectionError};
use diesel_async::{
    pooled_connection::AsyncDieselConnectionManager, AsyncConnection as _, AsyncPgConnection,
    RunQueryDsl,
};
use futures::Future;
use parking_lot::Mutex;
use uuid::Uuid;

use crate::{common::config::postgres::PrivilegedConfig, util::get_db_name};

use super::{
    super::{
        common::pool::diesel::r#trait::DieselPoolAssociation, error::Error as BackendError,
        r#trait::Backend,
    },
    r#trait::{PostgresBackend, PostgresBackendWrapper},
};

type CreateEntities = dyn Fn(AsyncPgConnection) -> Pin<Box<dyn Future<Output = AsyncPgConnection> + Send + 'static>>
    + Send
    + Sync
    + 'static;

pub struct DieselAsyncPgBackend<P>
where
    P: DieselPoolAssociation<AsyncPgConnection>,
{
    privileged_config: PrivilegedConfig,
    default_pool: P::Pool,
    db_conns: Mutex<HashMap<Uuid, AsyncPgConnection>>,
    create_restricted_pool: Box<dyn Fn() -> P::Builder + Send + Sync + 'static>,
    create_entities: Box<CreateEntities>,
    drop_previous_databases_flag: bool,
}

impl<P> DieselAsyncPgBackend<P>
where
    P: DieselPoolAssociation<AsyncPgConnection>,
{
    pub async fn new(
        privileged_config: PrivilegedConfig,
        create_privileged_pool: impl Fn() -> P::Builder,
        create_restricted_pool: impl Fn() -> P::Builder + Send + Sync + 'static,
        create_entities: impl Fn(
                AsyncPgConnection,
            ) -> Pin<Box<dyn Future<Output = AsyncPgConnection> + Send + 'static>>
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
impl<'pool, P> PostgresBackend<'pool> for DieselAsyncPgBackend<P>
where
    P: DieselPoolAssociation<AsyncPgConnection>,
{
    type Connection = AsyncPgConnection;
    type PooledConnection = P::PooledConnection<'pool>;
    type Pool = P::Pool;

    type BuildError = P::BuildError;
    type PoolError = P::PoolError;
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
        &'pool self,
    ) -> Result<P::PooledConnection<'pool>, P::PoolError> {
        P::get_connection(&self.default_pool).await
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

    async fn create_connection_pool(&self, db_id: Uuid) -> Result<P::Pool, P::BuildError> {
        let db_name = get_db_name(db_id);
        let db_name = db_name.as_str();
        let database_url = self.privileged_config.restricted_database_connection_url(
            db_name,
            Some(db_name),
            db_name,
        );
        let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url.as_str());
        let builder = (self.create_restricted_pool)();
        P::build_pool(builder, manager).await
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

type BError<BuildError, PoolError> = BackendError<BuildError, PoolError, ConnectionError, Error>;

#[async_trait]
impl<P> Backend for DieselAsyncPgBackend<P>
where
    P: DieselPoolAssociation<AsyncPgConnection>,
{
    type Pool = P::Pool;

    type BuildError = P::BuildError;
    type PoolError = P::PoolError;
    type ConnectionError = ConnectionError;
    type QueryError = Error;

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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::needless_return)]

    use bb8::Pool;
    use diesel::sql_query;
    use diesel_async::RunQueryDsl;
    use tokio_shared_rt::test;

    use crate::{
        common::config::PrivilegedPostgresConfig,
        r#async::backend::common::pool::diesel::bb8::DieselBb8,
    };

    use super::{
        super::r#trait::tests::{
            test_cleans_database, test_creates_database_with_restricted_privileges,
            test_drops_database, test_drops_previous_databases, CREATE_ENTITIES_STMT,
        },
        DieselAsyncPgBackend,
    };

    async fn create_backend(with_table: bool) -> DieselAsyncPgBackend<DieselBb8> {
        DieselAsyncPgBackend::new(
            PrivilegedPostgresConfig::new("postgres".to_owned())
                .password(Some("postgres".to_owned())),
            Pool::builder,
            Pool::builder,
            {
                move |mut conn| {
                    if with_table {
                        Box::pin(async move {
                            sql_query(CREATE_ENTITIES_STMT)
                                .execute(&mut conn)
                                .await
                                .unwrap();
                            conn
                        })
                    } else {
                        Box::pin(async { conn })
                    }
                }
            },
        )
        .await
        .unwrap()
    }

    #[test(shared)]
    async fn drops_previous_databases() {
        test_drops_previous_databases(
            create_backend(false).await,
            create_backend(false).await.drop_previous_databases(true),
            create_backend(false).await.drop_previous_databases(false),
        )
        .await;
    }

    #[test(shared)]
    async fn creates_database_with_restricted_privileges() {
        let backend = create_backend(true).await.drop_previous_databases(false);
        test_creates_database_with_restricted_privileges(backend).await;
    }

    #[test(shared)]
    async fn cleans_database() {
        let backend = create_backend(true).await.drop_previous_databases(false);
        test_cleans_database(backend).await;
    }

    #[test(shared)]
    async fn drops_database() {
        let backend = create_backend(true).await.drop_previous_databases(false);
        test_drops_database(backend).await;
    }
}
