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

/// ``SeaORM`` ``MySQL`` backend
pub struct SeaORMMySQLBackend {
    privileged_config: PrivilegedMySQLConfig,
    default_pool: DatabaseConnection,
    create_restricted_pool: Box<dyn for<'tmp> Fn(&'tmp mut ConnectOptions) + Send + Sync + 'static>,
    create_entities: Box<CreateEntities>,
    drop_previous_databases_flag: bool,
}

impl SeaORMMySQLBackend {
    /// Creates a new ``SeaORM`` ``MySQL`` backend
    /// # Example
    /// ```
    /// use db_pool::{r#async::SeaORMMySQLBackend, PrivilegedMySQLConfig};
    /// use dotenvy::dotenv;
    /// use sea_orm::ConnectionTrait;
    ///
    /// async fn f() {
    ///     dotenv().ok();
    ///
    ///     let config = PrivilegedMySQLConfig::from_env().unwrap();
    ///
    ///     let backend = SeaORMMySQLBackend::new(
    ///             config,
    ///             |opts| {
    ///                 opts.max_connections(10);
    ///             },
    ///             |opts| {
    ///                 opts.max_connections(2);
    ///             },
    ///             move |conn| {
    ///                 Box::pin(async move {
    ///                     conn.execute_unprepared(
    ///                         "CREATE TABLE book(id INTEGER PRIMARY KEY AUTO_INCREMENT, title TEXT NOT NULL)",
    ///                     )
    ///                     .await
    ///                     .unwrap();
    ///                 })
    ///             },
    ///         )
    ///         .await
    ///         .unwrap();
    /// }
    ///
    /// tokio_test::block_on(f());
    /// ```
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

    /// Drop databases created in previous runs upon initialization
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
        let chunks = query.into_iter().collect::<Vec<_>>();
        if chunks.is_empty() {
            Ok(())
        } else {
            let query = chunks.join(";");
            self.execute_stmt(query.as_str(), conn).await
        }
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::needless_return)]

    use futures::future::join_all;
    use sea_orm::{
        ActiveModelBehavior, ActiveModelTrait, ConnectionTrait, DeriveEntityModel,
        DerivePrimaryKey, DeriveRelation, EntityTrait, EnumIter, FromQueryResult, PaginatorTrait,
        PrimaryKeyTrait, QuerySelect, Set,
    };
    use tokio_shared_rt::test;

    use crate::{
        common::statement::mysql::tests::{
            CREATE_ENTITIES_STATEMENT, DDL_STATEMENTS, DML_STATEMENTS,
        },
        r#async::db_pool::DatabasePoolBuilder,
        tests::get_privileged_mysql_config,
    };

    use super::{
        super::r#trait::tests::{
            test_backend_cleans_database_with_tables, test_backend_cleans_database_without_tables,
            test_backend_creates_database_with_restricted_privileges, test_backend_drops_database,
            test_backend_drops_previous_databases, test_pool_drops_created_databases,
            test_pool_drops_previous_databases, MySQLDropLock,
        },
        SeaORMMySQLBackend,
    };

    #[derive(Clone, Debug, DeriveEntityModel)]
    #[sea_orm(table_name = "book")]
    pub struct Model {
        #[sea_orm(primary_key)]
        id: i32,
        title: String,
    }

    #[derive(Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}

    async fn create_backend(with_table: bool) -> SeaORMMySQLBackend {
        let config = get_privileged_mysql_config().clone();
        SeaORMMySQLBackend::new(config, |_| {}, |_| {}, {
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
        })
        .await
        .unwrap()
    }

    #[test(flavor = "multi_thread", shared)]
    async fn backend_drops_previous_databases() {
        test_backend_drops_previous_databases(
            create_backend(false).await,
            create_backend(false).await.drop_previous_databases(true),
            create_backend(false).await.drop_previous_databases(false),
        )
        .await;
    }

    #[test(flavor = "multi_thread", shared)]
    async fn backend_creates_database_with_restricted_privileges() {
        let backend = create_backend(true).await.drop_previous_databases(false);
        test_backend_creates_database_with_restricted_privileges(backend).await;
    }

    #[test(flavor = "multi_thread", shared)]
    async fn backend_cleans_database_with_tables() {
        let backend = create_backend(true).await.drop_previous_databases(false);
        test_backend_cleans_database_with_tables(backend).await;
    }

    #[test(flavor = "multi_thread", shared)]
    async fn backend_cleans_database_without_tables() {
        let backend = create_backend(false).await.drop_previous_databases(false);
        test_backend_cleans_database_without_tables(backend).await;
    }

    #[test(flavor = "multi_thread", shared)]
    async fn backend_drops_database() {
        let backend = create_backend(true).await.drop_previous_databases(false);
        test_backend_drops_database(backend).await;
    }

    #[test(flavor = "multi_thread", shared)]
    async fn pool_drops_previous_databases() {
        test_pool_drops_previous_databases(
            create_backend(false).await,
            create_backend(false).await.drop_previous_databases(true),
            create_backend(false).await.drop_previous_databases(false),
        )
        .await;
    }

    #[test(flavor = "multi_thread", shared)]
    async fn pool_provides_isolated_databases() {
        #[derive(FromQueryResult, Eq, PartialEq, Debug)]
        struct QueryModel {
            title: String,
        }

        const NUM_DBS: i64 = 3;

        let backend = create_backend(true).await.drop_previous_databases(false);

        async {
            let db_pool = backend.create_database_pool().await.unwrap();
            let conns = join_all((0..NUM_DBS).map(|_| db_pool.pull())).await;

            // insert single row into each database
            join_all(conns.iter().enumerate().map(|(i, conn)| async move {
                let book = ActiveModel {
                    title: Set(format!("Title {i}")),
                    ..Default::default()
                };
                book.insert(&***conn).await.unwrap();
            }))
            .await;

            // rows fetched must be as inserted
            join_all(conns.iter().enumerate().map(|(i, conn)| async move {
                assert_eq!(
                    Entity::find()
                        .select_only()
                        .column(Column::Title)
                        .into_model::<QueryModel>()
                        .all(&***conn)
                        .await
                        .unwrap(),
                    vec![QueryModel {
                        title: format!("Title {i}")
                    }]
                );
            }))
            .await;
        }
        .lock_read()
        .await;
    }

    #[test(flavor = "multi_thread", shared)]
    async fn pool_provides_restricted_databases() {
        let backend = create_backend(true).await.drop_previous_databases(false);

        async {
            let db_pool = backend.create_database_pool().await.unwrap();
            let conn = db_pool.pull().await;

            // DDL statements must fail
            for stmt in DDL_STATEMENTS {
                assert!(conn.execute_unprepared(stmt).await.is_err());
            }

            // DML statements must succeed
            for stmt in DML_STATEMENTS {
                assert!(conn.execute_unprepared(stmt).await.is_ok());
            }
        }
        .lock_read()
        .await;
    }

    #[test(flavor = "multi_thread", shared)]
    async fn pool_provides_clean_databases() {
        const NUM_DBS: i64 = 3;

        let backend = create_backend(true).await.drop_previous_databases(false);

        async {
            let db_pool = backend.create_database_pool().await.unwrap();

            // fetch connection pools the first time
            {
                let conns = join_all((0..NUM_DBS).map(|_| db_pool.pull())).await;

                // databases must be empty
                join_all(conns.iter().map(|conn| async move {
                    assert_eq!(Entity::find().count(&***conn).await.unwrap(), 0);
                }))
                .await;

                // insert data into each database
                join_all(conns.iter().map(|conn| async move {
                    let book = ActiveModel {
                        title: Set("Title".to_owned()),
                        ..Default::default()
                    };
                    book.insert(&***conn).await.unwrap();
                }))
                .await;
            }

            // fetch same connection pools a second time
            {
                let conns = join_all((0..NUM_DBS).map(|_| db_pool.pull())).await;

                // databases must be empty
                join_all(conns.iter().map(|conn| async move {
                    assert_eq!(Entity::find().count(&***conn).await.unwrap(), 0);
                }))
                .await;
            }
        }
        .lock_read()
        .await;
    }

    #[test(flavor = "multi_thread", shared)]
    async fn pool_drops_created_databases() {
        let backend = create_backend(false).await;
        test_pool_drops_created_databases(backend).await;
    }
}
