use std::{borrow::Cow, fmt::Debug, ops::Deref};

use r2d2::{ManageConnection, Pool, PooledConnection};
use uuid::Uuid;

use crate::common::statement::postgres;

use super::super::error::Error as BackendError;

pub(super) trait PostgresBackend {
    type ConnectionManager: ManageConnection;
    type ConnectionError: Into<BackendError<Self::ConnectionError, Self::QueryError>> + Debug;
    type QueryError: Into<BackendError<Self::ConnectionError, Self::QueryError>> + Debug;

    fn execute_query(
        &self,
        query: &str,
        conn: &mut <Self::ConnectionManager as ManageConnection>::Connection,
    ) -> Result<(), Self::QueryError>;
    fn batch_execute_query<'a>(
        &self,
        query: impl IntoIterator<Item = Cow<'a, str>>,
        conn: &mut <Self::ConnectionManager as ManageConnection>::Connection,
    ) -> Result<(), Self::QueryError>;

    fn get_default_connection(
        &self,
    ) -> Result<PooledConnection<Self::ConnectionManager>, r2d2::Error>;
    fn establish_privileged_database_connection(
        &self,
        db_id: Uuid,
    ) -> Result<<Self::ConnectionManager as ManageConnection>::Connection, Self::ConnectionError>;
    fn establish_restricted_database_connection(
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

pub(super) struct PostgresBackendWrapper<'a, B: PostgresBackend>(&'a B);

impl<'a, B: PostgresBackend> PostgresBackendWrapper<'a, B> {
    pub(super) fn new(backend: &'a B) -> Self {
        Self(backend)
    }
}

impl<'a, B: PostgresBackend> Deref for PostgresBackendWrapper<'a, B> {
    type Target = B;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'a, B: PostgresBackend> PostgresBackendWrapper<'a, B> {
    pub(super) fn init(&self) -> Result<(), BackendError<B::ConnectionError, B::QueryError>> {
        // Drop previous databases if needed
        if self.get_drop_previous_databases() {
            // Get default connection
            let conn = &mut self.get_default_connection()?;

            // Get previous database names
            let db_names = self.get_previous_database_names(conn).map_err(Into::into)?;

            // Drop databases
            for db_name in &db_names {
                self.execute_query(postgres::drop_database(db_name.as_str()).as_str(), conn)
                    .map_err(Into::into)?;
            }
        }

        Ok(())
    }

    #[allow(clippy::complexity)]
    pub(super) fn create(
        &self,
        db_id: uuid::Uuid,
        restrict_privileges: bool,
    ) -> Result<Pool<B::ConnectionManager>, BackendError<B::ConnectionError, B::QueryError>> {
        // Get database name based on UUID
        let db_name = crate::util::get_db_name(db_id);
        let db_name = db_name.as_str();

        {
            // Get connection to default database as privileged user
            let conn = &mut self.get_default_connection()?;

            // Create database
            self.execute_query(postgres::create_database(db_name).as_str(), conn)
                .map_err(Into::into)?;

            // Create role
            self.execute_query(postgres::create_role(db_name).as_str(), conn)
                .map_err(Into::into)?;
        }

        {
            // Connect to database as privileged user
            let mut conn = self
                .establish_privileged_database_connection(db_id)
                .map_err(Into::into)?;

            if restrict_privileges {
                // Create entities as privileged user
                self.create_entities(&mut conn);

                // Grant table privileges to restricted role
                self.execute_query(
                    postgres::grant_restricted_table_privileges(db_name).as_str(),
                    &mut conn,
                )
                .map_err(Into::into)?;

                // Grant sequence privileges to restricted role
                self.execute_query(
                    postgres::grant_restricted_sequence_privileges(db_name).as_str(),
                    &mut conn,
                )
                .map_err(Into::into)?;

                // Store database connection for reuse when cleaning
                self.put_database_connection(db_id, conn);
            } else {
                // Grant database ownership to database-unrestricted role
                self.execute_query(
                    postgres::grant_database_ownership(db_name, db_name).as_str(),
                    &mut conn,
                )
                .map_err(Into::into)?;

                // Connect to database as database-unrestricted user
                let mut conn = self
                    .establish_restricted_database_connection(db_id)
                    .map_err(Into::into)?;

                // Create entities as database-unrestricted user
                self.create_entities(&mut conn);
            }
        }

        // Create connection pool with attached role
        let pool = self.create_connection_pool(db_id)?;

        Ok(pool)
    }

    pub(super) fn clean(
        &self,
        db_id: uuid::Uuid,
    ) -> Result<(), BackendError<B::ConnectionError, B::QueryError>> {
        // Get privileged connection to database
        let mut conn = self.get_database_connection(db_id);

        // Get table names
        let table_names = self.get_table_names(&mut conn).map_err(Into::into)?;

        // Generate truncate statements
        let stmts = table_names
            .iter()
            .map(|table_name| postgres::truncate_table(table_name.as_str()).into());

        // Truncate tables
        self.batch_execute_query(stmts, &mut conn)
            .map_err(Into::into)?;

        // Store database connection back for reuse
        self.put_database_connection(db_id, conn);

        Ok(())
    }

    pub(super) fn drop(
        &self,
        db_id: uuid::Uuid,
        is_restricted: bool,
    ) -> Result<(), BackendError<B::ConnectionError, B::QueryError>> {
        // Drop privileged connection to database
        if is_restricted {
            self.get_database_connection(db_id);
        }

        // Get database name based on UUID
        let db_name = crate::util::get_db_name(db_id);
        let db_name = db_name.as_str();

        // Get connection to default database as privileged user
        let conn = &mut self.get_default_connection()?;

        // Drop database
        self.execute_query(postgres::drop_database(db_name).as_str(), conn)
            .map_err(Into::into)?;

        // Drop attached role
        self.execute_query(postgres::drop_role(db_name).as_str(), conn)
            .map_err(Into::into)?;

        Ok(())
    }
}

#[cfg(test)]
pub(super) mod tests {
    #![allow(unused_variables, clippy::unwrap_used)]

    use std::sync::OnceLock;

    use diesel::{
        dsl::exists, insert_into, prelude::*, r2d2::ConnectionManager, select, sql_query, table,
        PgConnection, RunQueryDsl,
    };
    use r2d2::Pool as R2d2Pool;
    use tokio::sync::{RwLockReadGuard, RwLockWriteGuard};
    use uuid::Uuid;

    use crate::{
        common::statement::postgres::tests::{DDL_STATEMENTS, DML_STATEMENTS},
        r#sync::{backend::r#trait::Backend, db_pool::DatabasePoolBuilder},
        tests::{get_privileged_postgres_config, PG_DROP_LOCK},
        util::get_db_name,
    };

    pub type Pool = R2d2Pool<ConnectionManager<PgConnection>>;

    table! {
        pg_database (oid) {
            oid -> Int4,
            datname -> Text
        }
    }

    fn lock_drop<'a>() -> RwLockWriteGuard<'a, ()> {
        PG_DROP_LOCK.blocking_write()
    }

    pub fn lock_read<'a>() -> RwLockReadGuard<'a, ()> {
        PG_DROP_LOCK.blocking_read()
    }

    fn get_privileged_connection_pool() -> &'static Pool {
        static POOL: OnceLock<Pool> = OnceLock::new();
        POOL.get_or_init(|| {
            let config = get_privileged_postgres_config();
            let database_url = config.default_connection_url();
            let manager = ConnectionManager::new(database_url);
            R2d2Pool::builder().build(manager).unwrap()
        })
    }

    fn create_restricted_connection_pool(db_name: &str) -> Pool {
        let config = get_privileged_postgres_config();
        let database_url =
            config.restricted_database_connection_url(db_name, Some(db_name), db_name);
        let manager = ConnectionManager::new(database_url);
        R2d2Pool::builder().build(manager).unwrap()
    }

    fn create_database(conn: &mut PgConnection) -> String {
        let db_id = Uuid::new_v4();
        let db_name = get_db_name(db_id);
        sql_query(format!("CREATE DATABASE {db_name}"))
            .execute(conn)
            .unwrap();
        db_name
    }

    fn create_databases(count: i64, conn: &mut PgConnection) -> Vec<String> {
        (0..count).map(|_| create_database(conn)).collect()
    }

    fn count_databases(db_names: &Vec<String>, conn: &mut PgConnection) -> i64 {
        pg_database::table
            .filter(pg_database::datname.eq_any(db_names))
            .count()
            .get_result(conn)
            .unwrap()
    }

    fn count_all_databases(conn: &mut PgConnection) -> i64 {
        pg_database::table
            .filter(pg_database::datname.like("db_pool_%"))
            .count()
            .get_result(conn)
            .unwrap()
    }

    fn database_exists(db_name: &str, conn: &mut PgConnection) -> bool {
        select(exists(
            pg_database::table.filter(pg_database::datname.eq(db_name)),
        ))
        .get_result(conn)
        .unwrap()
    }

    pub fn test_backend_drops_previous_databases<B: Backend>(default: B, enabled: B, disabled: B) {
        const NUM_DBS: i64 = 3;

        let conn_pool = get_privileged_connection_pool();
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

    pub fn test_backend_creates_database_with_restricted_privileges(backend: &impl Backend) {
        let db_id = Uuid::new_v4();
        let db_name = get_db_name(db_id);
        let db_name = db_name.as_str();

        let guard = lock_read();

        // privileged operations
        {
            let conn_pool = get_privileged_connection_pool();
            let conn = &mut conn_pool.get().unwrap();

            // database must not exist
            assert!(!database_exists(db_name, conn));

            // database must exist after creating through backend
            backend.init().unwrap();
            backend.create(db_id, true).unwrap();
            assert!(database_exists(db_name, conn));
        }

        // restricted operations
        {
            let conn_pool = &mut create_restricted_connection_pool(db_name);
            let conn = &mut conn_pool.get().unwrap();

            // DDL statements must fail
            for stmt in DDL_STATEMENTS {
                assert!(sql_query(stmt).execute(conn).is_err());
            }

            // DML statements must succeed
            for stmt in DML_STATEMENTS {
                assert!(sql_query(stmt).execute(conn).is_ok());
            }
        }
    }

    pub fn test_backend_creates_database_with_unrestricted_privileges(backend: &impl Backend) {
        let guard = lock_read();

        {
            let db_id = Uuid::new_v4();
            let db_name = get_db_name(db_id);
            let db_name = db_name.as_str();

            // privileged operations
            {
                let conn_pool = get_privileged_connection_pool();
                let conn = &mut conn_pool.get().unwrap();

                // database must not exist
                assert!(!database_exists(db_name, conn));

                // database must exist after creating through backend
                backend.init().unwrap();
                backend.create(db_id, false).unwrap();
                assert!(database_exists(db_name, conn));
            }

            // DML statements must succeed
            {
                let conn_pool = &mut create_restricted_connection_pool(db_name);
                let conn = &mut conn_pool.get().unwrap();
                for stmt in DML_STATEMENTS {
                    assert!(sql_query(stmt).execute(conn).is_ok());
                }
            }
        }

        // DDL statements must succeed
        for stmt in DDL_STATEMENTS {
            let db_id = Uuid::new_v4();
            let db_name = get_db_name(db_id);
            let db_name = db_name.as_str();

            backend.create(db_id, false).unwrap();
            let conn_pool = &mut create_restricted_connection_pool(db_name);
            let conn = &mut conn_pool.get().unwrap();

            assert!(sql_query(stmt).execute(conn).is_ok());
        }
    }

    pub fn test_backend_cleans_database_with_tables(backend: &impl Backend) {
        const NUM_BOOKS: i64 = 3;

        let db_id = Uuid::new_v4();
        let db_name = get_db_name(db_id);
        let db_name = db_name.as_str();

        let guard = lock_read();

        backend.init().unwrap();
        backend.create(db_id, true).unwrap();

        let conn_pool = &mut create_restricted_connection_pool(db_name);
        let conn = &mut conn_pool.get().unwrap();

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

    pub fn test_backend_cleans_database_without_tables(backend: &impl Backend) {
        let db_id = Uuid::new_v4();

        let guard = lock_read();

        backend.init().unwrap();
        backend.create(db_id, true).unwrap();
        backend.clean(db_id).unwrap();
    }

    pub fn test_backend_drops_database(backend: &impl Backend, restricted: bool) {
        let db_id = Uuid::new_v4();
        let db_name = get_db_name(db_id);
        let db_name = db_name.as_str();

        let conn_pool = get_privileged_connection_pool();
        let conn = &mut conn_pool.get().unwrap();

        let guard = lock_read();

        // database must exist
        backend.init().unwrap();
        backend.create(db_id, restricted).unwrap();
        assert!(database_exists(db_name, conn));

        // database must not exist
        backend.drop(db_id, restricted).unwrap();
        assert!(!database_exists(db_name, conn));
    }

    pub fn test_pool_drops_previous_databases<B: Backend>(default: B, enabled: B, disabled: B) {
        const NUM_DBS: i64 = 3;

        let guard = lock_drop();

        let conn_pool = get_privileged_connection_pool();
        let conn = &mut conn_pool.get().unwrap();

        for (backend, cleans) in [(default, true), (enabled, true), (disabled, false)] {
            let db_names = create_databases(NUM_DBS, conn);
            assert_eq!(count_databases(&db_names, conn), NUM_DBS);
            backend.create_database_pool().unwrap();
            assert_eq!(
                count_databases(&db_names, conn),
                if cleans { 0 } else { NUM_DBS }
            );
        }
    }

    pub fn test_pool_drops_created_restricted_databases(backend: impl Backend) {
        const NUM_DBS: i64 = 3;

        let conn_pool = get_privileged_connection_pool();
        let conn = &mut conn_pool.get().unwrap();

        let guard = lock_drop();

        let db_pool = backend.create_database_pool().unwrap();

        // there must be no databases
        assert_eq!(count_all_databases(conn), 0);

        // fetch connection pools
        let conn_pools = (0..NUM_DBS)
            .map(|_| db_pool.pull_immutable())
            .collect::<Vec<_>>();

        // there must be databases
        assert_eq!(count_all_databases(conn), NUM_DBS);

        // must release databases back to pool
        drop(conn_pools);

        // there must be databases
        assert_eq!(count_all_databases(conn), NUM_DBS);

        // must drop databases
        drop(db_pool);

        // there must be no databases
        assert_eq!(count_all_databases(conn), 0);
    }

    pub fn test_pool_drops_created_unrestricted_database(backend: impl Backend) {
        let conn_pool = get_privileged_connection_pool();
        let conn = &mut conn_pool.get().unwrap();

        let guard = lock_drop();

        let db_pool = backend.create_database_pool().unwrap();

        // there must be no databases
        assert_eq!(count_all_databases(conn), 0);

        // fetch connection pool
        let conn_pool = db_pool.create_mutable().unwrap();

        // there must be a database
        assert_eq!(count_all_databases(conn), 1);

        // must drop database
        drop(conn_pool);

        // there must be no databases
        assert_eq!(count_all_databases(conn), 0);

        drop(db_pool);

        // there must be no databases
        assert_eq!(count_all_databases(conn), 0);
    }
}
