use std::borrow::Cow;

use r2d2::{ManageConnection, Pool, PooledConnection};
use uuid::Uuid;

pub trait MySQLBackend {
    type ConnectionManager: ManageConnection;
    type ConnectionError;
    type QueryError;

    fn get_connection(&self) -> Result<PooledConnection<Self::ConnectionManager>, r2d2::Error>;

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

    fn get_host(&self) -> Cow<str>;

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
        db_name: &str,
        conn: &mut <Self::ConnectionManager as ManageConnection>::Connection,
    ) -> Result<Vec<String>, Self::QueryError>;

    fn get_drop_previous_databases(&self) -> bool;
}

macro_rules! impl_backend_for_mysql_backend {
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
                    // Get privileged connection
                    let conn = &mut self.get_connection()?;

                    // Get previous database names
                    self.execute(mysql::USE_DEFAULT_DATABASE, conn)?;
                    let db_names = self.get_previous_database_names(conn)?;

                    // Drop databases
                    for db_name in &db_names {
                        self.execute(
                            crate::common::statement::mysql::drop_database(db_name.as_str())
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

                let host = &self.get_host();

                // Get privileged connection
                let conn = &mut self.get_connection()?;

                // Create database
                self.execute(mysql::create_database(db_name).as_str(), conn)?;

                // Create CRUD user
                self.execute(mysql::create_user(db_name, host).as_str(), conn)?;

                // Create entities
                self.execute(mysql::use_database(db_name).as_str(), conn)?;
                self.create_entities(conn);
                self.execute(mysql::USE_DEFAULT_DATABASE, conn)?;

                // Grant privileges to CRUD role
                self.execute(mysql::grant_privileges(db_name, host).as_str(), conn)?;

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
                let db_name = crate::util::get_db_name(db_id);
                let db_name = db_name.as_str();

                let conn = &mut self.get_connection()?;

                let mut table_names = self.get_table_names(db_name, conn)?;
                let stmts = table_names
                    .drain(..)
                    .map(|table_name| mysql::truncate_table(table_name.as_str(), db_name).into());

                self.execute(mysql::TURN_OFF_FOREIGN_KEY_CHECKS, conn)?;
                self.batch_execute(stmts, conn)?;
                self.execute(mysql::TURN_ON_FOREIGN_KEY_CHECKS, conn)?;
                Ok(())
            }

            fn drop(
                &self,
                db_id: uuid::Uuid,
            ) -> Result<
                (),
                crate::sync::backend::error::Error<Self::ConnectionError, Self::QueryError>,
            > {
                // Get database name based on UUID
                let db_name = crate::util::get_db_name(db_id);
                let db_name = db_name.as_str();

                let host = &self.get_host();

                // Get privileged connection
                let conn = &mut self.get_connection()?;

                // Drop database
                self.execute(mysql::drop_database(db_name).as_str(), conn)?;

                // Drop CRUD user
                self.execute(mysql::drop_user(db_name, host).as_str(), conn)?;

                Ok(())
            }
        }
    };
}

pub(crate) use impl_backend_for_mysql_backend;
