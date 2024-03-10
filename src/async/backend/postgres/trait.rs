use std::borrow::Cow;

use async_trait::async_trait;
use bb8::{ManageConnection, Pool, PooledConnection, RunError};
use uuid::Uuid;

#[async_trait]
pub trait AsyncPgBackend: Sized + Send + Sync + 'static {
    type ConnectionManager: ManageConnection;
    type ConnectionError;
    type QueryError;

    async fn execute_stmt(
        &self,
        query: &str,
        conn: &mut <Self::ConnectionManager as ManageConnection>::Connection,
    ) -> Result<(), Self::QueryError>;
    async fn batch_execute_stmt<'a>(
        &self,
        query: impl IntoIterator<Item = Cow<'a, str>> + Send,
        conn: &mut <Self::ConnectionManager as ManageConnection>::Connection,
    ) -> Result<(), Self::QueryError>;

    async fn get_default_connection(
        &self,
    ) -> Result<
        PooledConnection<Self::ConnectionManager>,
        RunError<<Self::ConnectionManager as ManageConnection>::Error>,
    >;
    async fn establish_database_connection(
        &self,
        db_id: Uuid,
    ) -> Result<<Self::ConnectionManager as ManageConnection>::Connection, Self::ConnectionError>;
    fn put_database_connection(
        &self,
        db_id: Uuid,
        conn: <Self::ConnectionManager as ManageConnection>::Connection,
    );
    fn get_database_connection(
        &self,
        db_id: Uuid,
    ) -> <Self::ConnectionManager as ManageConnection>::Connection;

    async fn get_previous_database_names(
        &self,
        conn: &mut <Self::ConnectionManager as ManageConnection>::Connection,
    ) -> Result<Vec<String>, Self::QueryError>;
    async fn create_entities(
        &self,
        conn: <Self::ConnectionManager as ManageConnection>::Connection,
    ) -> <Self::ConnectionManager as ManageConnection>::Connection;
    async fn create_connection_pool(
        &self,
        db_id: Uuid,
    ) -> Result<
        Pool<Self::ConnectionManager>,
        RunError<<Self::ConnectionManager as ManageConnection>::Error>,
    >;

    async fn get_table_names(
        &self,
        privileged_conn: &mut <Self::ConnectionManager as ManageConnection>::Connection,
    ) -> Result<Vec<String>, Self::QueryError>;

    fn get_drop_previous_databases(&self) -> bool;
}

macro_rules! impl_async_backend_for_async_pg_backend {
    ($struct_name: ident, $manager: ident, $connection_error: ident, $query_error: ident) => {
        #[async_trait::async_trait]
        impl crate::r#async::backend::r#trait::Backend for $struct_name {
            type ConnectionManager = $manager;
            type ConnectionError = $connection_error;
            type QueryError = $query_error;

            async fn init(
                &self,
            ) -> Result<
                (),
                crate::r#async::backend::error::Error<
                    <Self::ConnectionManager as bb8::ManageConnection>::Error,
                    Self::ConnectionError,
                    Self::QueryError,
                >,
            > {
                // Drop previous databases if needed
                if self.get_drop_previous_databases() {
                    // Get connection to default database as privileged user
                    let conn = &mut self.get_default_connection().await?;

                    // Get previous database names
                    let db_names = self.get_previous_database_names(conn).await?;

                    // Drop databases
                    let futures = db_names
                        .iter()
                        .map(|db_name| async move {
                            let conn = &mut self.get_default_connection().await?;
                            self.execute_stmt(
                                crate::common::statement::postgres::drop_database(db_name.as_str())
                                    .as_str(),
                                conn,
                            )
                            .await?;
                            Ok::<
                                _,
                                crate::r#async::backend::error::Error<
                                    <Self::ConnectionManager as bb8::ManageConnection>::Error,
                                    Self::ConnectionError,
                                    Self::QueryError,
                                >,
                            >(())
                        })
                        .collect::<Vec<_>>();
                    futures::future::try_join_all(futures).await?;
                }

                Ok(())
            }

            async fn create(
                &self,
                db_id: uuid::Uuid,
            ) -> Result<
                Pool<Self::ConnectionManager>,
                crate::r#async::backend::error::Error<
                    <Self::ConnectionManager as bb8::ManageConnection>::Error,
                    Self::ConnectionError,
                    Self::QueryError,
                >,
            > {
                // Get database name based on UUID
                let db_name = crate::util::get_db_name(db_id);
                let db_name = db_name.as_str();

                {
                    // Get connection to default database as privileged user
                    let conn = &mut self.get_default_connection().await?;

                    // Create database
                    self.execute_stmt(
                        crate::common::statement::postgres::create_database(db_name).as_str(),
                        conn,
                    )
                    .await?;

                    // Create CRUD role
                    self.execute_stmt(
                        crate::common::statement::postgres::create_role(db_name).as_str(),
                        conn,
                    )
                    .await?;
                }

                {
                    // Connect to database as privileged user
                    let conn = self.establish_database_connection(db_id).await?;

                    // Create entities
                    let mut conn = self.create_entities(conn).await;

                    // Grant privileges to CRUD role
                    self.execute_stmt(
                        crate::common::statement::postgres::grant_table_privileges(db_name)
                            .as_str(),
                        &mut conn,
                    )
                    .await?;
                    self.execute_stmt(
                        crate::common::statement::postgres::grant_sequence_privileges(db_name)
                            .as_str(),
                        &mut conn,
                    )
                    .await?;

                    // Store database connection for reuse when cleaning
                    self.put_database_connection(db_id, conn);
                }

                // Create connection pool with CRUD role
                let pool = self.create_connection_pool(db_id).await?;
                Ok(pool)
            }

            async fn clean(
                &self,
                db_id: uuid::Uuid,
            ) -> Result<
                (),
                crate::r#async::backend::error::Error<
                    <Self::ConnectionManager as bb8::ManageConnection>::Error,
                    Self::ConnectionError,
                    Self::QueryError,
                >,
            > {
                let mut conn = self.get_database_connection(db_id);
                let table_names = self.get_table_names(&mut conn).await?;
                let stmts = table_names.iter().map(|table_name| {
                    crate::common::statement::postgres::truncate_table(table_name.as_str()).into()
                });
                self.batch_execute_stmt(stmts, &mut conn).await?;
                self.put_database_connection(db_id, conn);
                Ok(())
            }

            async fn drop(
                &self,
                db_id: uuid::Uuid,
            ) -> Result<
                (),
                crate::r#async::backend::error::Error<
                    <Self::ConnectionManager as bb8::ManageConnection>::Error,
                    Self::ConnectionError,
                    Self::QueryError,
                >,
            > {
                // Drop privileged connection to database
                {
                    self.get_database_connection(db_id);
                }

                // Get database name based on UUID
                let db_name = crate::util::get_db_name(db_id);
                let db_name = db_name.as_str();

                // Get connection to default database as privileged user
                let conn = &mut self.get_default_connection().await?;

                // Drop database
                self.execute_stmt(
                    crate::common::statement::postgres::drop_database(db_name).as_str(),
                    conn,
                )
                .await?;

                // Drop CRUD role
                self.execute_stmt(
                    crate::common::statement::postgres::drop_role(db_name).as_str(),
                    conn,
                )
                .await?;

                Ok(())
            }
        }
    };
}

pub(crate) use impl_async_backend_for_async_pg_backend;
