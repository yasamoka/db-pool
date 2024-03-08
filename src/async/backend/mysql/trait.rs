use std::borrow::Cow;

use async_trait::async_trait;
use bb8::{ManageConnection, Pool, PooledConnection};
use uuid::Uuid;

#[async_trait]
pub trait AsyncMySQLBackend {
    type ConnectionManager: ManageConnection;

    async fn get_connection(&self) -> PooledConnection<Self::ConnectionManager>;

    async fn execute_stmt(
        &self,
        query: &str,
        conn: &mut <Self::ConnectionManager as ManageConnection>::Connection,
    );
    async fn batch_execute_stmt<'a>(
        &self,
        query: impl IntoIterator<Item = Cow<'a, str>> + Send,
        conn: &mut <Self::ConnectionManager as ManageConnection>::Connection,
    );

    fn get_host(&self) -> &str;

    async fn create_entities(&self, db_name: &str);
    async fn create_connection_pool(&self, db_id: Uuid) -> Pool<Self::ConnectionManager>;

    async fn get_table_names(
        &self,
        db_name: &str,
        conn: &mut <Self::ConnectionManager as ManageConnection>::Connection,
    ) -> Vec<String>;

    async fn get_database_connection_ids(
        &self,
        db_name: &str,
        host: &str,
        conn: &mut <Self::ConnectionManager as ManageConnection>::Connection,
    ) -> Vec<i64>;
    fn terminate_connections(&self) -> bool;
}

macro_rules! impl_async_backend_for_async_mysql_backend {
    ($struct_name: ident, $manager: ident) => {
        #[async_trait::async_trait]
        impl crate::r#async::backend::r#trait::AsyncBackend for $struct_name {
            type ConnectionManager = $manager;

            async fn init(&self) {}

            async fn create(&self, db_id: uuid::Uuid) -> Pool<Self::ConnectionManager> {
                // Get database name based on UUID
                let db_name = crate::util::get_db_name(db_id);
                let db_name = db_name.as_str();

                let host = self.get_host();

                // Get privileged connection
                let conn = &mut self.get_connection().await;

                // Create database
                self.execute_stmt(mysql::create_database(db_name).as_str(), conn)
                    .await;

                // Create CRUD user
                self.execute_stmt(mysql::create_user(db_name, host).as_str(), conn)
                    .await;

                // Create entities
                self.execute_stmt(mysql::use_database(db_name).as_str(), conn)
                    .await;
                self.create_entities(db_name).await;
                self.execute_stmt(mysql::USE_DEFAULT_DATABASE, conn).await;

                // Grant privileges to CRUD role
                self.execute_stmt(mysql::grant_privileges(db_name, host).as_str(), conn)
                    .await;

                // Create connection pool with CRUD role
                self.create_connection_pool(db_id).await
            }

            async fn clean(&self, db_id: uuid::Uuid) {
                let db_name = crate::util::get_db_name(db_id);
                let db_name = db_name.as_str();

                let conn = &mut self.get_connection().await;

                let mut table_names = self.get_table_names(db_name, conn).await;
                let stmts = table_names
                    .drain(..)
                    .map(|table_name| mysql::truncate_table(table_name.as_str(), db_name).into());

                self.execute_stmt(mysql::TURN_OFF_FOREIGN_KEY_CHECKS, conn)
                    .await;
                self.batch_execute_stmt(stmts, conn).await;
                self.execute_stmt(mysql::TURN_ON_FOREIGN_KEY_CHECKS, conn)
                    .await;
            }

            async fn drop(&self, db_id: uuid::Uuid) {
                // Get database name based on UUID
                let db_name = crate::util::get_db_name(db_id);
                let db_name = db_name.as_str();

                let host = self.get_host();

                // Get privileged connection
                let conn = &mut self.get_connection().await;

                // Terminate all connections to database if needed
                if self.terminate_connections() {
                    let mut db_conn_ids =
                        self.get_database_connection_ids(db_name, host, conn).await;
                    let stmts = db_conn_ids
                        .drain(..)
                        .map(|conn_id| mysql::terminate_database_connection(conn_id).into());
                    self.batch_execute_stmt(stmts, conn).await;
                }

                // Drop database
                self.execute_stmt(mysql::drop_database(db_name).as_str(), conn)
                    .await;

                // Drop CRUD role
                self.execute_stmt(mysql::drop_user(db_name, host).as_str(), conn)
                    .await;
            }
        }
    };
}

pub(crate) use impl_async_backend_for_async_mysql_backend;
