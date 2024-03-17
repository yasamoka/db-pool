use std::borrow::Cow;

use r2d2::{ManageConnection, Pool, PooledConnection};
use uuid::Uuid;

pub(super) trait MySQLBackend {
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

#[cfg(test)]
pub(super) mod tests {
    #![allow(unused_variables, clippy::unwrap_used)]

    use diesel::{
        dsl::exists, insert_into, prelude::*, r2d2::ConnectionManager, select, sql_query, table,
        MysqlConnection, RunQueryDsl,
    };
    use r2d2::Pool as R2d2Pool;
    use tokio::sync::{RwLockReadGuard, RwLockWriteGuard};
    use uuid::Uuid;

    use crate::{
        common::statement::mysql::tests::{DDL_STATEMENTS, DML_STATEMENTS},
        r#sync::backend::r#trait::Backend,
        tests::MYSQL_DROP_LOCK,
        util::get_db_name,
    };

    pub type Pool = R2d2Pool<ConnectionManager<MysqlConnection>>;

    table! {
        schemata (schema_name) {
            schema_name -> Text
        }
    }

    fn lock_drop<'a>() -> RwLockWriteGuard<'a, ()> {
        MYSQL_DROP_LOCK.blocking_write()
    }

    fn lock_read<'a>() -> RwLockReadGuard<'a, ()> {
        MYSQL_DROP_LOCK.blocking_read()
    }

    fn create_privileged_connection_pool() -> Pool {
        let manager = ConnectionManager::new("mysql://root:root@localhost:3306");
        R2d2Pool::builder().build(manager).unwrap()
    }

    fn create_restricted_connection_pool(db_name: &str) -> Pool {
        let manager = ConnectionManager::new(format!(
            "mysql://{db_name}:{db_name}@localhost:3306/{db_name}"
        ));
        R2d2Pool::builder().build(manager).unwrap()
    }

    fn create_database(conn: &mut MysqlConnection) -> String {
        let db_id = Uuid::new_v4();
        let db_name = get_db_name(db_id);
        sql_query(format!("CREATE DATABASE {db_name}"))
            .execute(conn)
            .unwrap();
        db_name
    }

    fn create_databases(count: i64, conn: &mut MysqlConnection) -> Vec<String> {
        (0..count).map(|_| create_database(conn)).collect()
    }

    fn use_database(db_name: &str, conn: &mut MysqlConnection) {
        sql_query(format!("USE {db_name}")).execute(conn).unwrap();
    }

    fn use_information_schema(conn: &mut MysqlConnection) {
        use_database("information_schema", conn);
    }

    fn count_databases(db_names: &Vec<String>, conn: &mut MysqlConnection) -> i64 {
        use_information_schema(conn);

        schemata::table
            .filter(schemata::schema_name.eq_any(db_names))
            .count()
            .get_result(conn)
            .unwrap()
    }

    fn database_exists(db_name: &str, conn: &mut MysqlConnection) -> bool {
        use_information_schema(conn);

        select(exists(
            schemata::table.filter(schemata::schema_name.eq(db_name)),
        ))
        .get_result(conn)
        .unwrap()
    }

    pub fn test_drops_previous_databases<B>(default: B, enabled: B, disabled: B)
    where
        B: Backend,
    {
        const NUM_DBS: i64 = 3;

        let conn_pool = create_privileged_connection_pool();
        let conn = &mut conn_pool.get().unwrap();

        let guard = lock_drop();

        for (backend, cleans) in [(default, true), (enabled, true), (disabled, false)] {
            let db_names = create_databases(NUM_DBS, conn);
            assert_eq!(count_databases(&db_names, conn), NUM_DBS);
            backend.init().unwrap();
            assert_eq!(
                count_databases(&db_names, conn),
                if cleans { 0 } else { NUM_DBS }
            );
        }
    }

    pub fn test_creates_database_with_restricted_privileges(backend: &impl Backend) {
        let db_id = Uuid::new_v4();
        let db_name = get_db_name(db_id);
        let db_name = db_name.as_str();

        let guard = lock_read();

        // privileged operations
        {
            let conn_pool = create_privileged_connection_pool();
            let conn = &mut conn_pool.get().unwrap();

            // database must not exist
            assert!(!database_exists(db_name, conn));

            // database must exist after creating through backend
            backend.init().unwrap();
            backend.create(db_id).unwrap();
            assert!(database_exists(db_name, conn));
        }

        // restricted operations
        {
            let conn_pool = create_restricted_connection_pool(db_name);
            let conn = &mut conn_pool.get().unwrap();

            // // DDL statements must fail
            for stmt in DDL_STATEMENTS {
                assert!(sql_query(stmt).execute(conn).is_err());
            }

            // DML statements must succeed
            for stmt in DML_STATEMENTS {
                assert!(sql_query(stmt).execute(conn).is_ok());
            }
        }
    }

    pub fn test_cleans_database(backend: &impl Backend) {
        const NUM_BOOKS: i64 = 3;

        let db_id = Uuid::new_v4();
        let db_name = get_db_name(db_id);
        let db_name = db_name.as_str();

        let guard = lock_read();

        backend.init().unwrap();
        backend.create(db_id).unwrap();

        let conn_pool = create_privileged_connection_pool();
        let conn = &mut conn_pool.get().unwrap();

        use_database(db_name, conn);

        table! {
            book (id) {
                id -> Int4,
                title -> Text
            }
        }

        #[derive(Insertable)]
        #[diesel(table_name = book)]
        struct NewBook {
            title: String,
        }

        let new_books = (0..NUM_BOOKS)
            .map(|i| NewBook {
                title: format!("Title {}", i + 1),
            })
            .collect::<Vec<_>>();
        insert_into(book::table)
            .values(&new_books)
            .execute(conn)
            .unwrap();

        // there must be books
        assert_eq!(
            book::table.count().get_result::<i64>(conn).unwrap(),
            NUM_BOOKS
        );

        backend.clean(db_id).unwrap();

        // there must be no books
        assert_eq!(book::table.count().get_result::<i64>(conn).unwrap(), 0);
    }

    pub fn test_drops_database(backend: &impl Backend) {
        let db_id = Uuid::new_v4();
        let db_name = get_db_name(db_id);
        let db_name = db_name.as_str();

        let guard = lock_read();

        let conn_pool = create_privileged_connection_pool();
        let conn = &mut conn_pool.get().unwrap();

        // database must exist
        backend.init().unwrap();
        backend.create(db_id).unwrap();
        assert!(database_exists(db_name, conn));

        // database must not exist
        backend.drop(db_id).unwrap();
        assert!(!database_exists(db_name, conn));
    }
}
