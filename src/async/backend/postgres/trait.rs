use std::borrow::Cow;

use async_trait::async_trait;
use bb8::{ManageConnection, Pool, PooledConnection};
use uuid::Uuid;

#[async_trait]
pub trait AsyncPgBackend: Sized + Send + Sync + 'static {
    type ConnectionManager: ManageConnection;

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

    async fn get_default_connection(&self) -> PooledConnection<Self::ConnectionManager>;
    async fn establish_database_connection(
        &self,
        db_id: Uuid,
    ) -> <Self::ConnectionManager as ManageConnection>::Connection;
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
    ) -> Vec<String>;
    async fn create_entities(
        &self,
        conn: <Self::ConnectionManager as ManageConnection>::Connection,
    ) -> <Self::ConnectionManager as ManageConnection>::Connection;
    async fn create_connection_pool(&self, db_id: Uuid) -> Pool<Self::ConnectionManager>;

    async fn get_table_names(
        &self,
        privileged_conn: &mut <Self::ConnectionManager as ManageConnection>::Connection,
    ) -> Vec<String>;

    fn get_drop_previous_databases(&self) -> bool;
}

macro_rules! impl_async_backend_for_async_pg_backend {
    ($struct_name: ident, $manager: ident) => {
        #[async_trait::async_trait]
        impl crate::r#async::backend::r#trait::AsyncBackend for $struct_name {
            type ConnectionManager = $manager;

            async fn init(&self) {
                // Drop previous databases if needed
                if self.get_drop_previous_databases() {
                    // Get connection to default database as privileged user
                    let conn = &mut self.get_default_connection().await;

                    // Get previous database names
                    let mut db_names = self.get_previous_database_names(conn).await;

                    // Drop databases
                    let futures = db_names
                        .drain(..)
                        .map(|db_name| async move {
                            let conn = &mut self.get_default_connection().await;
                            self.execute_stmt(
                                crate::statement::pg::drop_database(db_name.as_str()).as_str(),
                                conn,
                            )
                            .await;
                        })
                        .collect::<Vec<_>>();
                    futures::future::join_all(futures).await;
                }
            }

            async fn create(&self, db_id: uuid::Uuid) -> Pool<Self::ConnectionManager> {
                // Get database name based on UUID
                let db_name = crate::util::get_db_name(db_id);
                let db_name = db_name.as_str();

                {
                    // Get connection to default database as privileged user
                    let conn = &mut self.get_default_connection().await;

                    // Create database
                    self.execute_stmt(
                        crate::statement::pg::create_database(db_name).as_str(),
                        conn,
                    )
                    .await;

                    // Create CRUD role
                    self.execute_stmt(crate::statement::pg::create_role(db_name).as_str(), conn)
                        .await;
                }

                {
                    // Connect to database as privileged user
                    let conn = self.establish_database_connection(db_id).await;

                    // Create entities
                    let mut conn = self.create_entities(conn).await;

                    // Grant privileges to CRUD role
                    self.execute_stmt(
                        crate::statement::pg::grant_table_privileges(db_name).as_str(),
                        &mut conn,
                    )
                    .await;
                    self.execute_stmt(
                        crate::statement::pg::grant_sequence_privileges(db_name).as_str(),
                        &mut conn,
                    )
                    .await;

                    // Store database connection for reuse when cleaning
                    self.put_database_connection(db_id, conn);
                }

                // Create connection pool with CRUD role
                self.create_connection_pool(db_id).await
            }

            async fn clean(&self, db_id: uuid::Uuid) {
                let mut conn = self.get_database_connection(db_id);
                let mut table_names = self.get_table_names(&mut conn).await;
                let stmts = table_names.drain(..).map(|table_name| {
                    crate::statement::pg::truncate_table(table_name.as_str()).into()
                });
                self.batch_execute_stmt(stmts, &mut conn).await;
                self.put_database_connection(db_id, conn);
            }

            async fn drop(&self, db_id: uuid::Uuid) {
                // Drop privileged connection to database
                {
                    self.get_database_connection(db_id);
                }

                // Get database name based on UUID
                let db_name = crate::util::get_db_name(db_id);
                let db_name = db_name.as_str();

                // Get connection to default database as privileged user
                let conn = &mut self.get_default_connection().await;

                // Drop database
                self.execute_stmt(crate::statement::pg::drop_database(db_name).as_str(), conn)
                    .await;

                // Drop CRUD role
                self.execute_stmt(crate::statement::pg::drop_role(db_name).as_str(), conn)
                    .await;
            }
        }
    };
}

pub(crate) use impl_async_backend_for_async_pg_backend;
