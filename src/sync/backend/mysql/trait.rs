use std::borrow::Cow;

use r2d2::{ManageConnection, Pool, PooledConnection};
use uuid::Uuid;

pub trait MySQLBackend {
    type ConnectionManager: ManageConnection;

    fn get_connection(&self) -> PooledConnection<Self::ConnectionManager>;

    fn execute(
        &self,
        query: &str,
        conn: &mut <Self::ConnectionManager as ManageConnection>::Connection,
    );
    fn batch_execute<'a>(
        &self,
        query: impl IntoIterator<Item = Cow<'a, str>>,
        conn: &mut <Self::ConnectionManager as ManageConnection>::Connection,
    );

    fn get_host(&self) -> &str;

    fn create_entities(&self, conn: &mut <Self::ConnectionManager as ManageConnection>::Connection);
    fn create_connection_pool(&self, db_id: Uuid) -> Pool<Self::ConnectionManager>;

    fn get_table_names(
        &self,
        db_name: &str,
        conn: &mut <Self::ConnectionManager as ManageConnection>::Connection,
    ) -> Vec<String>;

    fn get_database_connection_ids(
        &self,
        db_name: &str,
        host: &str,
        conn: &mut <Self::ConnectionManager as ManageConnection>::Connection,
    ) -> Vec<i64>;
    fn terminate_connections(&self) -> bool;
}

macro_rules! impl_backend_for_mysql_backend {
    ($struct_name: ident, $manager: ident) => {
        impl crate::sync::backend::r#trait::Backend for $struct_name {
            type ConnectionManager = $manager;

            fn create(&self, db_id: uuid::Uuid) -> Pool<Self::ConnectionManager> {
                // Get database name based on UUID
                let db_name = crate::util::get_db_name(db_id);
                let db_name = db_name.as_str();

                let host = self.get_host();

                // Get privileged connection
                let conn = &mut self.get_connection();

                // Create database
                self.execute(mysql::create_database(db_name).as_str(), conn);

                // Create CRUD user
                self.execute(mysql::create_user(db_name, host).as_str(), conn);

                // Create entities
                self.execute(mysql::use_database(db_name).as_str(), conn);
                self.create_entities(conn);
                self.execute(mysql::USE_DEFAULT_DATABASE, conn);

                // Grant privileges to CRUD role
                self.execute(mysql::grant_privileges(db_name, host).as_str(), conn);

                // Create connection pool with CRUD role
                self.create_connection_pool(db_id)
            }

            fn clean(&self, db_id: uuid::Uuid) {
                let db_name = crate::util::get_db_name(db_id);
                let db_name = db_name.as_str();

                let conn = &mut self.get_connection();

                let mut table_names = self.get_table_names(db_name, conn);
                let stmts = table_names
                    .drain(..)
                    .map(|table_name| mysql::truncate_table(table_name.as_str(), db_name).into());

                self.execute(mysql::TURN_OFF_FOREIGN_KEY_CHECKS, conn);
                self.batch_execute(stmts, conn);
                self.execute(mysql::TURN_ON_FOREIGN_KEY_CHECKS, conn);
            }

            fn drop(&self, db_id: uuid::Uuid) {
                // Get database name based on UUID
                let db_name = crate::util::get_db_name(db_id);
                let db_name = db_name.as_str();

                let host = self.get_host();

                // Get privileged connection
                let conn = &mut self.get_connection();

                // Terminate all connections to database if needed
                if self.terminate_connections() {
                    let mut db_conn_ids = self.get_database_connection_ids(db_name, host, conn);
                    let stmts = db_conn_ids
                        .drain(..)
                        .map(|conn_id| mysql::terminate_database_connection(conn_id).into());
                    self.batch_execute(stmts, conn);
                }

                // Drop database
                self.execute(mysql::drop_database(db_name).as_str(), conn);

                // Drop CRUD role
                self.execute(mysql::drop_user(db_name, host).as_str(), conn);
            }
        }
    };
}

pub(crate) use impl_backend_for_mysql_backend;
