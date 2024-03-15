use std::{borrow::Cow, pin::Pin};

use async_trait::async_trait;
use futures::Future;
use sea_orm::{
    ActiveModelBehavior, ColumnTrait, ConnectOptions, ConnectionTrait, Database,
    DatabaseConnection, DbErr, DeriveEntityModel, DerivePrimaryKey, DeriveRelation, EntityTrait,
    EnumIter, FromQueryResult, PrimaryKeyTrait, QueryFilter, QuerySelect, TransactionError,
    TransactionTrait,
};
use uuid::Uuid;

use crate::{
    common::{config::PrivilegedMySQLConfig, statement::mysql},
    util::get_db_name,
};

use super::{
    super::{
        common::{
            conn::sea_orm::PooledConnection,
            error::sea_orm::{BuildError, ConnectionError, PoolError, QueryError},
        },
        error::Error as BackendError,
        r#trait::Backend,
    },
    r#trait::{MySQLBackend, MySQLBackendWrapper},
};

type CreateEntities = dyn Fn(DatabaseConnection) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
    + Send
    + Sync
    + 'static;

pub struct SeaORMMySQLBackend {
    privileged_config: PrivilegedMySQLConfig,
    default_pool: DatabaseConnection,
    create_restricted_pool: Box<dyn for<'tmp> Fn(&'tmp mut ConnectOptions) + Send + Sync + 'static>,
    create_entities: Box<CreateEntities>,
    drop_previous_databases_flag: bool,
}

impl SeaORMMySQLBackend {
    pub async fn new(
        privileged_config: PrivilegedMySQLConfig,
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
impl<'pool> MySQLBackend<'pool> for SeaORMMySQLBackend {
    type Connection = DatabaseConnection;
    type PooledConnection = PooledConnection;
    type Pool = DatabaseConnection;

    type BuildError = BuildError;
    type PoolError = PoolError;
    type ConnectionError = ConnectionError;
    type QueryError = QueryError;

    async fn get_connection(&'pool self) -> Result<PooledConnection, PoolError> {
        Ok(self.default_pool.clone().into())
    }

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

    fn get_host(&self) -> &str {
        self.privileged_config.host.as_str()
    }

    async fn get_previous_database_names(
        &self,
        conn: &mut DatabaseConnection,
    ) -> Result<Vec<String>, QueryError> {
        #[derive(Clone, Debug, DeriveEntityModel)]
        #[sea_orm(table_name = "schemata")]
        pub struct Model {
            #[sea_orm(primary_key)]
            schema_name: String,
        }

        #[derive(Debug, EnumIter, DeriveRelation)]
        pub enum Relation {}

        impl ActiveModelBehavior for ActiveModel {}

        conn.transaction(move |txn| {
            Box::pin(async move {
                txn.execute_unprepared(mysql::USE_DEFAULT_DATABASE).await?;

                Entity::find()
                    .filter(Column::SchemaName.like("db_pool_%"))
                    .all(txn)
                    .await
            })
        })
        .await
        .map(|mut models| models.drain(..).map(|model| model.schema_name).collect())
        .map_err(|err| match err {
            TransactionError::Connection(err) | TransactionError::Transaction(err) => err.into(),
        })
    }

    async fn create_entities(&self, db_name: &str) -> Result<(), ConnectionError> {
        let database_url = self
            .privileged_config
            .privileged_database_connection_url(db_name);
        let conn = Database::connect(database_url).await?;
        (self.create_entities)(conn).await;
        Ok(())
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

    // TODO: improve error in trait to include both query and connection errors
    async fn get_table_names(
        &self,
        db_name: &str,
        conn: &mut DatabaseConnection,
    ) -> Result<Vec<String>, QueryError> {
        #[derive(Clone, Debug, DeriveEntityModel)]
        #[sea_orm(table_name = "tables")]
        pub struct Model {
            #[sea_orm(primary_key)]
            table_name: String,
            table_schema: String,
        }

        #[derive(Debug, EnumIter, DeriveRelation)]
        pub enum Relation {}

        impl ActiveModelBehavior for ActiveModel {}

        #[derive(FromQueryResult)]
        struct QueryModel {
            table_name: String,
        }

        conn.transaction(move |txn| {
            let db_name = db_name.to_owned();
            Box::pin(async move {
                txn.execute_unprepared(mysql::USE_DEFAULT_DATABASE).await?;

                Entity::find()
                    .select_only()
                    .column(Column::TableName)
                    .filter(Column::TableSchema.eq(db_name))
                    .into_model::<QueryModel>()
                    .all(txn)
                    .await
            })
        })
        .await
        .map(|mut models| models.drain(..).map(|model| model.table_name).collect())
        .map_err(|err| match err {
            TransactionError::Connection(err) | TransactionError::Transaction(err) => err.into(),
        })
    }

    fn get_drop_previous_databases(&self) -> bool {
        self.drop_previous_databases_flag
    }
}

type BError = BackendError<BuildError, PoolError, ConnectionError, QueryError>;

#[async_trait]
impl Backend for SeaORMMySQLBackend {
    type Pool = DatabaseConnection;

    type BuildError = BuildError;
    type PoolError = PoolError;
    type ConnectionError = ConnectionError;
    type QueryError = QueryError;

    async fn init(&self) -> Result<(), BError> {
        MySQLBackendWrapper::new(self).init().await
    }

    async fn create(&self, db_id: uuid::Uuid) -> Result<DatabaseConnection, BError> {
        MySQLBackendWrapper::new(self).create(db_id).await
    }

    async fn clean(&self, db_id: uuid::Uuid) -> Result<(), BError> {
        MySQLBackendWrapper::new(self).clean(db_id).await
    }

    async fn drop(&self, db_id: uuid::Uuid) -> Result<(), BError> {
        MySQLBackendWrapper::new(self).drop(db_id).await
    }
}
