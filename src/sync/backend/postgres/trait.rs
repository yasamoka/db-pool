use std::borrow::Cow;

use r2d2::{ManageConnection, Pool, PooledConnection};
use uuid::Uuid;

pub(super) trait PostgresBackend {
    type ConnectionManager: ManageConnection;
    type ConnectionError;
    type QueryError;

    fn execute(
        &self,
        query: &str,
        conn: &mut <Self::ConnectionManager as ManageConnection>::Connection,
    ) -> Result<(), Self::QueryError>;
    fn batch_execute<'a>(
        &self,
        query: impl IntoIterator<Item = Cow<'a, str>>,
        conn: &mut <Self::ConnectionManager as ManageConnection>::Connection,
    ) -> Result<(), Self::QueryError>;

    fn get_default_connection(
        &self,
    ) -> Result<PooledConnection<Self::ConnectionManager>, r2d2::Error>;
    fn establish_database_connection(
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

    fn get_previous_database_names(
        &self,
        conn: &mut <Self::ConnectionManager as ManageConnection>::Connection,
    ) -> Result<Vec<String>, Self::QueryError>;
    fn create_entities(&self, conn: &mut <Self::ConnectionManager as ManageConnection>::Connection);
    fn create_connection_pool(
        &self,
        db_id: Uuid,
    ) -> Result<Pool<Self::ConnectionManager>, r2d2::Error>;

    fn get_table_names(
        &self,
        conn: &mut <Self::ConnectionManager as ManageConnection>::Connection,
    ) -> Result<Vec<String>, Self::QueryError>;

    fn get_drop_previous_databases(&self) -> bool;
}

macro_rules! impl_backend_for_pg_backend {
    ($struct_name: ident, $manager: ident, $connection_error: ident, $query_error: ident) => {
        impl crate::sync::backend::r#trait::Backend for $struct_name {
            type ConnectionManager = $manager;
            type ConnectionError = $connection_error;
            type QueryError = $query_error;

            fn init(
                &self,
            ) -> Result<
                (),
                crate::sync::backend::error::Error<Self::ConnectionError, Self::QueryError>,
            > {
                // Drop previous databases if needed
                if self.get_drop_previous_databases() {
                    // Get default connection
                    let conn = &mut self.get_default_connection()?;

                    // Get previous database names
                    let db_names = self.get_previous_database_names(conn)?;

                    // Drop databases
                    for db_name in &db_names {
                        self.execute(
                            crate::common::statement::postgres::drop_database(db_name.as_str())
                                .as_str(),
                            conn,
                        )?;
                    }
                }

                Ok(())
            }

            fn create(
                &self,
                db_id: uuid::Uuid,
            ) -> Result<
                Pool<Self::ConnectionManager>,
                crate::sync::backend::error::Error<Self::ConnectionError, Self::QueryError>,
            > {
                // Get database name based on UUID
                let db_name = crate::util::get_db_name(db_id);
                let db_name = db_name.as_str();

                {
                    // Get connection to default database as privileged user
                    let conn = &mut self.get_default_connection()?;

                    // Create database
                    self.execute(
                        crate::common::statement::postgres::create_database(db_name).as_str(),
                        conn,
                    )?;

                    // Create CRUD role
                    self.execute(
                        crate::common::statement::postgres::create_role(db_name).as_str(),
                        conn,
                    )?;
                }

                {
                    // Connect to database as privileged user
                    let mut conn = self.establish_database_connection(db_id)?;

                    // Create entities
                    self.create_entities(&mut conn);

                    // Grant privileges to CRUD role
                    self.execute(
                        crate::common::statement::postgres::grant_table_privileges(db_name)
                            .as_str(),
                        &mut conn,
                    )?;
                    self.execute(
                        crate::common::statement::postgres::grant_sequence_privileges(db_name)
                            .as_str(),
                        &mut conn,
                    )?;

                    // Store database connection for reuse when cleaning
                    self.put_database_connection(db_id, conn);
                }

                // Create connection pool with CRUD role
                let pool = self.create_connection_pool(db_id)?;
                Ok(pool)
            }

            fn clean(
                &self,
                db_id: uuid::Uuid,
            ) -> Result<
                (),
                crate::sync::backend::error::Error<Self::ConnectionError, Self::QueryError>,
            > {
                let mut conn = self.get_database_connection(db_id);
                let table_names = self.get_table_names(&mut conn)?;
                let stmts = table_names.iter().map(|table_name| {
                    crate::common::statement::postgres::truncate_table(table_name.as_str()).into()
                });
                self.batch_execute(stmts, &mut conn)?;
                self.put_database_connection(db_id, conn);
                Ok(())
            }

            fn drop(
                &self,
                db_id: uuid::Uuid,
            ) -> Result<
                (),
                crate::sync::backend::error::Error<Self::ConnectionError, Self::QueryError>,
            > {
                // Drop privileged connection to database
                {
                    self.get_database_connection(db_id);
                }

                // Get database name based on UUID
                let db_name = crate::util::get_db_name(db_id);
                let db_name = db_name.as_str();

                // Get connection to default database as privileged user
                let conn = &mut self.get_default_connection()?;

                // Drop database
                self.execute(
                    crate::common::statement::postgres::drop_database(db_name).as_str(),
                    conn,
                )?;

                // Drop CRUD role
                self.execute(
                    crate::common::statement::postgres::drop_role(db_name).as_str(),
                    conn,
                )?;

                Ok(())
            }
        }
    };
}

pub(crate) use impl_backend_for_pg_backend;
