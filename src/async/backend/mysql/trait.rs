use std::borrow::Cow;

use async_trait::async_trait;
use bb8::{ManageConnection, Pool, PooledConnection};
use uuid::Uuid;

#[async_trait]
pub trait AsyncMySQLBackend {
    type ConnectionManager: ManageConnection;
    type ConnectionError;
    type QueryError;

    async fn get_connection(
        &self,
    ) -> Result<
        PooledConnection<Self::ConnectionManager>,
        bb8::RunError<<Self::ConnectionManager as ManageConnection>::Error>,
    >;

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

    fn get_host(&self) -> &str;

    async fn get_previous_database_names(
        &self,
        conn: &mut <Self::ConnectionManager as ManageConnection>::Connection,
    ) -> Result<Vec<String>, Self::QueryError>;
    // TODO: think of a better error type here
    async fn create_entities(&self, db_name: &str) -> Result<(), Self::ConnectionError>;
    async fn create_connection_pool(
        &self,
        db_id: Uuid,
    ) -> Result<
        Pool<Self::ConnectionManager>,
        bb8::RunError<<Self::ConnectionManager as ManageConnection>::Error>,
    >;

    async fn get_table_names(
        &self,
        db_name: &str,
        conn: &mut <Self::ConnectionManager as ManageConnection>::Connection,
    ) -> Result<Vec<String>, Self::QueryError>;

    fn get_drop_previous_databases(&self) -> bool;
}

macro_rules! impl_async_backend_for_async_mysql_backend {
    ($struct_name: ident, $manager: ident, $connection_error: ident, $query_error: ident) => {
        #[async_trait::async_trait]
        impl crate::r#async::backend::r#trait::AsyncBackend for $struct_name {
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
                    // Get privileged connection
                    let conn = &mut self.get_connection().await?;

                    // Get previous database names
                    self.execute_stmt(mysql::USE_DEFAULT_DATABASE, conn).await?;
                    let mut db_names = self.get_previous_database_names(conn).await?;

                    // Drop databases
                    let futures = db_names
                        .drain(..)
                        .map(|db_name| async move {
                            let conn = &mut self.get_connection().await?;
                            self.execute_stmt(
                                crate::statement::mysql::drop_database(db_name.as_str()).as_str(),
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

                let host = self.get_host();

                // Get privileged connection
                let conn = &mut self.get_connection().await?;

                // Create database
                self.execute_stmt(mysql::create_database(db_name).as_str(), conn)
                    .await?;

                // Create CRUD user
                self.execute_stmt(mysql::create_user(db_name, host).as_str(), conn)
                    .await?;

                // Create entities
                self.execute_stmt(mysql::use_database(db_name).as_str(), conn)
                    .await?;
                self.create_entities(db_name).await?;
                self.execute_stmt(mysql::USE_DEFAULT_DATABASE, conn).await?;

                // Grant privileges to CRUD role
                self.execute_stmt(mysql::grant_privileges(db_name, host).as_str(), conn)
                    .await?;

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
                let db_name = crate::util::get_db_name(db_id);
                let db_name = db_name.as_str();

                let conn = &mut self.get_connection().await?;

                let table_names = self.get_table_names(db_name, conn).await?;
                let stmts = table_names
                    .iter()
                    .map(|table_name| mysql::truncate_table(table_name.as_str(), db_name).into());

                self.execute_stmt(mysql::TURN_OFF_FOREIGN_KEY_CHECKS, conn)
                    .await?;
                self.batch_execute_stmt(stmts, conn).await?;
                self.execute_stmt(mysql::TURN_ON_FOREIGN_KEY_CHECKS, conn)
                    .await?;
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
                // Get database name based on UUID
                let db_name = crate::util::get_db_name(db_id);
                let db_name = db_name.as_str();

                let host = self.get_host();

                // Get privileged connection
                let conn = &mut self.get_connection().await?;

                // Drop database
                self.execute_stmt(mysql::drop_database(db_name).as_str(), conn)
                    .await?;

                // Drop CRUD role
                self.execute_stmt(mysql::drop_user(db_name, host).as_str(), conn)
                    .await?;

                Ok(())
            }
        }
    };
}

pub(crate) use impl_async_backend_for_async_mysql_backend;
