use std::borrow::Cow;

use r2d2::{ManageConnection, Pool, PooledConnection};
use uuid::Uuid;

pub trait PgBackend {
    type ConnectionManager: ManageConnection;

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

    fn get_default_connection(&self) -> PooledConnection<Self::ConnectionManager>;
    fn establish_database_connection(
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

    fn create_entities(&self, conn: &mut <Self::ConnectionManager as ManageConnection>::Connection);
    fn create_connection_pool(&self, db_id: Uuid) -> Pool<Self::ConnectionManager>;

    fn get_table_names(
        &self,
        privileged_conn: &mut <Self::ConnectionManager as ManageConnection>::Connection,
    ) -> Vec<String>;

    fn terminate_connections(&self) -> bool;
}

macro_rules! impl_backend_for_pg_backend {
    ($struct_name: ident, $manager: ident) => {
        impl crate::sync::backend::r#trait::Backend for $struct_name {
            type ConnectionManager = $manager;

            fn create(&self, db_id: uuid::Uuid) -> Pool<Self::ConnectionManager> {
                // Get database name based on UUID
                let db_name = crate::util::get_db_name(db_id);
                let db_name = db_name.as_str();

                {
                    // Get connection to default database as privileged user
                    let conn = &mut self.get_default_connection();

                    // Create database
                    self.execute(
                        crate::statement::pg::create_database(db_name).as_str(),
                        conn,
                    );

                    // Create CRUD role
                    self.execute(crate::statement::pg::create_role(db_name).as_str(), conn);
                }

                {
                    // Connect to database as privileged user
                    let mut conn = self.establish_database_connection(db_id);

                    // Create entities
                    self.create_entities(&mut conn);

                    // Grant privileges to CRUD role
                    self.execute(
                        crate::statement::pg::grant_privileges(db_name).as_str(),
                        &mut conn,
                    );

                    // Store database connection for reuse when cleaning
                    self.put_database_connection(db_id, conn);
                }

                // Create connection pool with CRUD role
                self.create_connection_pool(db_id)
            }

            fn clean(&self, db_id: uuid::Uuid) {
                let mut conn = self.get_database_connection(db_id);
                let mut table_names = self.get_table_names(&mut conn);
                let stmts = table_names.drain(..).map(|table_name| {
                    crate::statement::pg::truncate_table(table_name.as_str()).into()
                });
                self.batch_execute(stmts, &mut conn);
                self.put_database_connection(db_id, conn);
            }

            fn drop(&self, db_id: uuid::Uuid) {
                // Drop privileged connection to database
                {
                    self.get_database_connection(db_id);
                }

                // Get database name based on UUID
                let db_name = crate::util::get_db_name(db_id);
                let db_name = db_name.as_str();

                // Get connection to default database as privileged user
                let conn = &mut self.get_default_connection();

                // Terminate all connections to database if needed
                if self.terminate_connections() {
                    self.execute(
                        crate::statement::pg::terminate_database_connections(db_name).as_str(),
                        conn,
                    );
                }

                // Drop database
                self.execute(crate::statement::pg::drop_database(db_name).as_str(), conn);

                // Drop CRUD role
                self.execute(crate::statement::pg::drop_role(db_name).as_str(), conn);
            }
        }
    };
}

pub(crate) use impl_backend_for_pg_backend;
