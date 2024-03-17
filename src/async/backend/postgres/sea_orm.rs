use std::{borrow::Cow, collections::HashMap, pin::Pin};

use async_trait::async_trait;
use futures::Future;
use parking_lot::Mutex;
use sea_orm::{
    ActiveModelBehavior, ColumnTrait, ConnectOptions, ConnectionTrait, Database,
    DatabaseConnection, DbErr, DeriveEntityModel, DerivePrimaryKey, DeriveRelation, EntityTrait,
    EnumIter, FromQueryResult, PrimaryKeyTrait, QueryFilter, QuerySelect,
};
use uuid::Uuid;

use crate::{common::config::PrivilegedPostgresConfig, util::get_db_name};

use super::{
    super::{
        common::{
            conn::sea_orm::PooledConnection,
            error::sea_orm::{BuildError, ConnectionError, PoolError, QueryError},
        },
        error::Error as BackendError,
        r#trait::Backend,
    },
    r#trait::{PostgresBackend, PostgresBackendWrapper},
};

type CreateEntities = dyn Fn(DatabaseConnection) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
    + Send
    + Sync
    + 'static;

pub struct SeaORMPostgresBackend {
    privileged_config: PrivilegedPostgresConfig,
    default_pool: DatabaseConnection,
    db_conns: Mutex<HashMap<Uuid, DatabaseConnection>>,
    create_restricted_pool: Box<dyn for<'tmp> Fn(&'tmp mut ConnectOptions) + Send + Sync + 'static>,
    create_entities: Box<CreateEntities>,
    drop_previous_databases_flag: bool,
}

impl SeaORMPostgresBackend {
    pub async fn new(
        privileged_config: PrivilegedPostgresConfig,
        create_privileged_pool: impl for<'tmp> Fn(&'tmp mut ConnectOptions),
        create_restricted_pool: impl for<'tmp> Fn(&'tmp mut ConnectOptions) + Send + Sync + 'static,
        create_entities: impl Fn(DatabaseConnection) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
            + Send
            + Sync
            + 'static,
    ) -> Result<Self, DbErr> {
        let mut opts = ConnectOptions::new(privileged_config.default_connection_url());
        create_privileged_pool(&mut opts);
        let default_pool = Database::connect(opts).await?;

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
impl<'pool> PostgresBackend<'pool> for SeaORMPostgresBackend {
    type Connection = DatabaseConnection;
    type PooledConnection = PooledConnection;
    type Pool = DatabaseConnection;

    type BuildError = BuildError;
    type PoolError = PoolError;
    type ConnectionError = ConnectionError;
    type QueryError = QueryError;

    async fn execute_stmt(
        &self,
        query: &str,
        conn: &mut DatabaseConnection,
    ) -> Result<(), QueryError> {
        conn.execute_unprepared(query).await?;
        Ok(())
    }

    async fn batch_execute_stmt<'a>(
        &self,
        query: impl IntoIterator<Item = Cow<'a, str>> + Send,
        conn: &mut DatabaseConnection,
    ) -> Result<(), QueryError> {
        let query = query.into_iter().collect::<Vec<_>>().join(";");
        self.execute_stmt(query.as_str(), conn).await
    }

    async fn get_default_connection(&'pool self) -> Result<PooledConnection, PoolError> {
        Ok(self.default_pool.clone().into())
    }

    async fn establish_database_connection(
        &self,
        db_id: Uuid,
    ) -> Result<DatabaseConnection, ConnectionError> {
        let db_name = get_db_name(db_id);
        let database_url = self
            .privileged_config
            .privileged_database_connection_url(db_name.as_str());
        let opts = ConnectOptions::new(database_url);
        Database::connect(opts).await.map_err(Into::into)
    }

    fn put_database_connection(&self, db_id: Uuid, conn: DatabaseConnection) {
        self.db_conns.lock().insert(db_id, conn);
    }

    fn get_database_connection(&self, db_id: Uuid) -> DatabaseConnection {
        self.db_conns
            .lock()
            .remove(&db_id)
            .unwrap_or_else(|| panic!("connection map must have a connection for {db_id}"))
    }

    async fn get_previous_database_names(
        &self,
        conn: &mut DatabaseConnection,
    ) -> Result<Vec<String>, QueryError> {
        #[derive(Clone, Debug, DeriveEntityModel)]
        #[sea_orm(table_name = "pg_database")]
        pub struct Model {
            #[sea_orm(primary_key)]
            oid: i32,
            datname: String,
        }

        #[derive(Debug, EnumIter, DeriveRelation)]
        pub enum Relation {}

        impl ActiveModelBehavior for ActiveModel {}

        #[derive(FromQueryResult)]
        struct QueryModel {
            datname: String,
        }

        Entity::find()
            .select_only()
            .column(Column::Datname)
            .filter(Column::Datname.like("db_pool_%"))
            .into_model::<QueryModel>()
            .all(conn)
            .await
            .map(|mut models| models.drain(..).map(|model| model.datname).collect())
            .map_err(Into::into)
    }

    async fn create_entities(&self, conn: DatabaseConnection) -> DatabaseConnection {
        (self.create_entities)(conn.clone()).await;
        conn
    }

    async fn create_connection_pool(&self, db_id: Uuid) -> Result<DatabaseConnection, BuildError> {
        let db_name = get_db_name(db_id);
        let db_name = db_name.as_str();
        let database_url = self.privileged_config.restricted_database_connection_url(
            db_name,
            Some(db_name),
            db_name,
        );
        let mut opts = ConnectOptions::new(database_url);
        (self.create_restricted_pool)(&mut opts);
        Database::connect(opts).await.map_err(Into::into)
    }

    async fn get_table_names(
        &self,
        conn: &mut DatabaseConnection,
    ) -> Result<Vec<String>, QueryError> {
        #[derive(Clone, Debug, DeriveEntityModel)]
        #[sea_orm(table_name = "pg_tables")]
        pub struct Model {
            schemaname: String,
            #[sea_orm(primary_key)]
            tablename: String,
        }

        #[derive(Debug, EnumIter, DeriveRelation)]
        pub enum Relation {}

        impl ActiveModelBehavior for ActiveModel {}

        #[derive(FromQueryResult)]
        struct QueryModel {
            tablename: String,
        }

        Entity::find()
            .select_only()
            .column(Column::Tablename)
            .filter(Column::Schemaname.is_not_in(["pg_catalog", "information_schema"]))
            .into_model::<QueryModel>()
            .all(conn)
            .await
            .map(|mut models| models.drain(..).map(|model| model.tablename).collect())
            .map_err(Into::into)
    }

    fn get_drop_previous_databases(&self) -> bool {
        self.drop_previous_databases_flag
    }
}

type BError = BackendError<BuildError, PoolError, ConnectionError, QueryError>;

#[async_trait]
impl Backend for SeaORMPostgresBackend {
    type Pool = DatabaseConnection;

    type BuildError = BuildError;
    type PoolError = PoolError;
    type ConnectionError = ConnectionError;
    type QueryError = QueryError;

    async fn init(&self) -> Result<(), BError> {
        PostgresBackendWrapper::new(self).init().await
    }

    async fn create(&self, db_id: uuid::Uuid) -> Result<DatabaseConnection, BError> {
        PostgresBackendWrapper::new(self).create(db_id).await
    }

    async fn clean(&self, db_id: uuid::Uuid) -> Result<(), BError> {
        PostgresBackendWrapper::new(self).clean(db_id).await
    }

    async fn drop(&self, db_id: uuid::Uuid) -> Result<(), BError> {
        PostgresBackendWrapper::new(self).drop(db_id).await
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::needless_return)]

    use sea_orm::ConnectionTrait;
    use tokio_shared_rt::test;

    use crate::common::{
        config::PrivilegedPostgresConfig, statement::postgres::tests::CREATE_ENTITIES_STATEMENT,
    };

    use super::{
        super::r#trait::tests::{
            test_cleans_database, test_creates_database_with_restricted_privileges,
            test_drops_database, test_drops_previous_databases,
        },
        SeaORMPostgresBackend,
    };

    async fn create_backend(with_table: bool) -> SeaORMPostgresBackend {
        SeaORMPostgresBackend::new(
            PrivilegedPostgresConfig::new("postgres".to_owned())
                .password(Some("postgres".to_owned())),
            |_| {},
            |_| {},
            {
                move |conn| {
                    if with_table {
                        Box::pin(async move {
                            conn.execute_unprepared(CREATE_ENTITIES_STATEMENT)
                                .await
                                .unwrap();
                        })
                    } else {
                        Box::pin(async {})
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
