use std::{borrow::Cow, collections::HashMap};

use diesel::{
    QueryResult, RunQueryDsl, connection::SimpleConnection, pg::PgConnection, prelude::*,
    r2d2::ConnectionManager, result::Error, sql_query,
};
use parking_lot::Mutex;
use r2d2::{Builder, Pool, PooledConnection};
use uuid::Uuid;

use crate::{common::config::postgres::PrivilegedPostgresConfig, util::get_db_name};

use super::{
    super::{error::Error as BackendError, r#trait::Backend},
    r#trait::{PostgresBackend, PostgresBackendWrapper},
};

type Manager = ConnectionManager<PgConnection>;

/// [`Diesel Postgres`](https://docs.rs/diesel/2.2.11/diesel/pg/struct.PgConnection.html) backend
pub struct DieselPostgresBackend {
    privileged_config: PrivilegedPostgresConfig,
    default_pool: Pool<Manager>,
    db_conns: Mutex<HashMap<Uuid, PgConnection>>,
    create_restricted_pool: Box<dyn Fn() -> Builder<Manager> + Send + Sync + 'static>,
    create_entities: Box<dyn Fn(&mut PgConnection) + Send + Sync + 'static>,
    drop_previous_databases_flag: bool,
}

impl DieselPostgresBackend {
    /// Creates a new [`Diesel Postgres`](https://docs.rs/diesel/2.2.11/diesel/pg/struct.PgConnection.html) backend
    /// # Example
    /// ```
    /// use db_pool::{sync::DieselPostgresBackend, PrivilegedPostgresConfig};
    /// use diesel::{sql_query, RunQueryDsl};
    /// use dotenvy::dotenv;
    /// use r2d2::Pool;
    ///
    /// dotenv().ok();
    ///
    /// let config = PrivilegedPostgresConfig::from_env().unwrap();
    ///
    /// let backend = DieselPostgresBackend::new(
    ///     config,
    ///     || Pool::builder().max_size(10),
    ///     || Pool::builder().max_size(2),
    ///     move |conn| {
    ///         sql_query("CREATE TABLE book(id SERIAL PRIMARY KEY, title TEXT NOT NULL)")
    ///             .execute(conn)
    ///             .unwrap();
    ///     },
    /// )
    /// .unwrap();
    /// ```
    pub fn new(
        privileged_config: PrivilegedPostgresConfig,
        create_privileged_pool: impl Fn() -> Builder<Manager>,
        create_restricted_pool: impl Fn() -> Builder<Manager> + Send + Sync + 'static,
        create_entities: impl Fn(&mut PgConnection) + Send + Sync + 'static,
    ) -> Result<Self, r2d2::Error> {
        let manager = Manager::new(privileged_config.default_connection_url());
        let default_pool = (create_privileged_pool()).build(manager)?;

        Ok(Self {
            privileged_config,
            default_pool,
            db_conns: Mutex::new(HashMap::new()),
            create_entities: Box::new(create_entities),
            create_restricted_pool: Box::new(create_restricted_pool),
            drop_previous_databases_flag: true,
        })
    }

    /// Drop databases created in previous runs upon initialization
    #[must_use]
    pub fn drop_previous_databases(self, value: bool) -> Self {
        Self {
            drop_previous_databases_flag: value,
            ..self
        }
    }
}

impl PostgresBackend for DieselPostgresBackend {
    type ConnectionManager = Manager;
    type ConnectionError = ConnectionError;
    type QueryError = Error;

    fn execute_query(&self, query: &str, conn: &mut PgConnection) -> QueryResult<()> {
        sql_query(query).execute(conn)?;
        Ok(())
    }

    fn batch_execute_query<'a>(
        &self,
        query: impl IntoIterator<Item = Cow<'a, str>>,
        conn: &mut PgConnection,
    ) -> QueryResult<()> {
        let query = query.into_iter().collect::<Vec<_>>();
        if query.is_empty() {
            Ok(())
        } else {
            conn.batch_execute(query.join(";").as_str())
        }
    }

    fn get_default_connection(&self) -> Result<PooledConnection<Manager>, r2d2::Error> {
        self.default_pool.get()
    }

    fn establish_privileged_database_connection(
        &self,
        db_id: Uuid,
    ) -> ConnectionResult<PgConnection> {
        let db_name = get_db_name(db_id);
        let database_url = self
            .privileged_config
            .privileged_database_connection_url(db_name.as_str());
        PgConnection::establish(database_url.as_str())
    }

    fn establish_restricted_database_connection(
        &self,
        db_id: Uuid,
    ) -> ConnectionResult<PgConnection> {
        let db_name = get_db_name(db_id);
        let db_name = db_name.as_str();
        let database_url = self.privileged_config.restricted_database_connection_url(
            db_name,
            Some(db_name),
            db_name,
        );
        PgConnection::establish(database_url.as_str())
    }

    fn put_database_connection(&self, db_id: Uuid, conn: PgConnection) {
        self.db_conns.lock().insert(db_id, conn);
    }

    fn get_database_connection(&self, db_id: Uuid) -> PgConnection {
        self.db_conns
            .lock()
            .remove(&db_id)
            .unwrap_or_else(|| panic!("connection map must have a connection for {db_id}"))
    }

    fn get_previous_database_names(&self, conn: &mut PgConnection) -> QueryResult<Vec<String>> {
        table! {
            pg_database (oid) {
                oid -> Int4,
                datname -> Text
            }
        }

        pg_database::table
            .select(pg_database::datname)
            .filter(pg_database::datname.like("db_pool_%"))
            .load::<String>(conn)
    }

    fn create_entities(&self, conn: &mut PgConnection) {
        (self.create_entities)(conn);
    }

    fn create_connection_pool(
        &self,
        db_id: Uuid,
    ) -> Result<Pool<Self::ConnectionManager>, r2d2::Error> {
        let db_name = get_db_name(db_id);
        let db_name = db_name.as_str();
        let database_url = self.privileged_config.restricted_database_connection_url(
            db_name,
            Some(db_name),
            db_name,
        );
        let manager = ConnectionManager::<PgConnection>::new(database_url.as_str());
        (self.create_restricted_pool)().build(manager)
    }

    fn get_table_names(&self, conn: &mut PgConnection) -> QueryResult<Vec<String>> {
        table! {
            pg_tables (tablename) {
                #[sql_name = "schemaname"]
                schema_name -> Text,
                tablename -> Text
            }
        }

        pg_tables::table
            .filter(pg_tables::schema_name.ne_all(["pg_catalog", "information_schema"]))
            .select(pg_tables::tablename)
            .load(conn)
    }

    fn get_drop_previous_databases(&self) -> bool {
        self.drop_previous_databases_flag
    }
}

impl Backend for DieselPostgresBackend {
    type ConnectionManager = Manager;
    type ConnectionError = ConnectionError;
    type QueryError = Error;

    fn init(&self) -> Result<(), BackendError<ConnectionError, Error>> {
        PostgresBackendWrapper::new(self).init()
    }

    fn create(
        &self,
        db_id: Uuid,
        restrict_privileges: bool,
    ) -> Result<Pool<Manager>, BackendError<ConnectionError, Error>> {
        PostgresBackendWrapper::new(self).create(db_id, restrict_privileges)
    }

    fn clean(&self, db_id: Uuid) -> Result<(), BackendError<ConnectionError, Error>> {
        PostgresBackendWrapper::new(self).clean(db_id)
    }

    fn drop(
        &self,
        db_id: Uuid,
        is_restricted: bool,
    ) -> Result<(), BackendError<ConnectionError, Error>> {
        PostgresBackendWrapper::new(self).drop(db_id, is_restricted)
    }
}

#[cfg(test)]
mod tests {
    #![allow(unused_variables, clippy::unwrap_used, clippy::needless_return)]

    use std::borrow::Cow;

    use diesel::{
        Insertable, QueryDsl, RunQueryDsl, connection::SimpleConnection, insert_into, sql_query,
        table,
    };
    use dotenvy::dotenv;
    use r2d2::Pool;

    use crate::{
        common::{
            config::PrivilegedPostgresConfig,
            statement::postgres::tests::{
                CREATE_ENTITIES_STATEMENTS, DDL_STATEMENTS, DML_STATEMENTS,
            },
        },
        sync::{
            backend::postgres::r#trait::tests::test_backend_creates_database_with_unrestricted_privileges,
            db_pool::DatabasePoolBuilder,
        },
    };

    use super::{
        super::r#trait::tests::{
            lock_read, test_backend_cleans_database_with_tables,
            test_backend_cleans_database_without_tables,
            test_backend_creates_database_with_restricted_privileges, test_backend_drops_database,
            test_backend_drops_previous_databases, test_pool_drops_created_restricted_databases,
            test_pool_drops_created_unrestricted_database, test_pool_drops_previous_databases,
        },
        DieselPostgresBackend,
    };

    table! {
        book (id) {
            id -> Int4,
            title -> Text
        }
    }

    #[derive(Insertable)]
    #[diesel(table_name = book)]
    struct NewBook<'a> {
        title: Cow<'a, str>,
    }

    fn create_backend(with_table: bool) -> DieselPostgresBackend {
        dotenv().ok();

        let config = PrivilegedPostgresConfig::from_env().unwrap();

        DieselPostgresBackend::new(config, Pool::builder, Pool::builder, {
            move |conn| {
                if with_table {
                    let query = CREATE_ENTITIES_STATEMENTS.join(";");
                    conn.batch_execute(query.as_str()).unwrap();
                }
            }
        })
        .unwrap()
    }

    #[test]
    fn backend_drops_previous_databases() {
        test_backend_drops_previous_databases(
            create_backend(false),
            create_backend(false).drop_previous_databases(true),
            create_backend(false).drop_previous_databases(false),
        );
    }

    #[test]
    fn backend_creates_database_with_restricted_privileges() {
        let backend = create_backend(true).drop_previous_databases(false);
        test_backend_creates_database_with_restricted_privileges(&backend);
    }

    #[test]
    fn backend_creates_database_with_unrestricted_privileges() {
        let backend = create_backend(true).drop_previous_databases(false);
        test_backend_creates_database_with_unrestricted_privileges(&backend);
    }

    #[test]
    fn backend_cleans_database_with_tables() {
        let backend = create_backend(true).drop_previous_databases(false);
        test_backend_cleans_database_with_tables(&backend);
    }

    #[test]
    fn backend_cleans_database_without_tables() {
        let backend = create_backend(false).drop_previous_databases(false);
        test_backend_cleans_database_without_tables(&backend);
    }

    #[test]
    fn backend_drops_restricted_database() {
        let backend = create_backend(true).drop_previous_databases(false);
        test_backend_drops_database(&backend, true);
    }

    #[test]
    fn backend_drops_unrestricted_database() {
        let backend = create_backend(true).drop_previous_databases(false);
        test_backend_drops_database(&backend, false);
    }

    #[test]
    fn pool_drops_previous_databases() {
        test_pool_drops_previous_databases(
            create_backend(false),
            create_backend(false).drop_previous_databases(true),
            create_backend(false).drop_previous_databases(false),
        );
    }

    #[test]
    fn pool_provides_isolated_databases() {
        const NUM_DBS: i64 = 3;

        let backend = create_backend(true).drop_previous_databases(false);

        let guard = lock_read();

        let db_pool = backend.create_database_pool().unwrap();
        let conn_pools = (0..NUM_DBS)
            .map(|_| db_pool.pull_immutable())
            .collect::<Vec<_>>();

        // insert single row into each database
        conn_pools.iter().enumerate().for_each(|(i, conn_pool)| {
            let conn = &mut conn_pool.get().unwrap();
            insert_into(book::table)
                .values(NewBook {
                    title: format!("Title {i}").into(),
                })
                .execute(conn)
                .unwrap();
        });

        // rows fetched must be as inserted
        conn_pools.iter().enumerate().for_each(|(i, conn_pool)| {
            let conn = &mut conn_pool.get().unwrap();
            assert_eq!(
                book::table
                    .select(book::title)
                    .load::<String>(conn)
                    .unwrap(),
                vec![format!("Title {i}")]
            );
        });
    }

    #[test]
    fn pool_provides_restricted_databases() {
        let backend = create_backend(true).drop_previous_databases(false);

        let guard = lock_read();

        let db_pool = backend.create_database_pool().unwrap();
        let conn_pool = db_pool.pull_immutable();
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

    #[test]
    fn pool_provides_unrestricted_databases() {
        let backend = create_backend(true).drop_previous_databases(false);

        let guard = lock_read();

        let db_pool = backend.create_database_pool().unwrap();

        // DML statements must succeed
        {
            let conn_pool = db_pool.create_mutable().unwrap();
            let conn = &mut conn_pool.get().unwrap();
            for stmt in DML_STATEMENTS {
                assert!(sql_query(stmt).execute(conn).is_ok());
            }
        }

        // DDL statements must succeed
        for stmt in DDL_STATEMENTS {
            let conn_pool = db_pool.create_mutable().unwrap();
            let conn = &mut conn_pool.get().unwrap();
            assert!(sql_query(stmt).execute(conn).is_ok());
        }
    }

    #[test]
    fn pool_provides_clean_databases() {
        const NUM_DBS: i64 = 3;

        let backend = create_backend(true).drop_previous_databases(false);

        let guard = lock_read();

        let db_pool = backend.create_database_pool().unwrap();

        // fetch connection pools the first time
        {
            let conn_pools = (0..NUM_DBS)
                .map(|_| db_pool.pull_immutable())
                .collect::<Vec<_>>();

            // databases must be empty
            for conn_pool in &conn_pools {
                let conn = &mut conn_pool.get().unwrap();
                assert_eq!(book::table.count().get_result::<i64>(conn).unwrap(), 0);
            }

            // insert data into each database
            for conn_pool in &conn_pools {
                let conn = &mut conn_pool.get().unwrap();
                insert_into(book::table)
                    .values(NewBook {
                        title: "Title".into(),
                    })
                    .execute(conn)
                    .unwrap();
            }
        }

        // fetch same connection pools a second time
        {
            let conn_pools = (0..NUM_DBS)
                .map(|_| db_pool.pull_immutable())
                .collect::<Vec<_>>();

            // databases must be empty
            for conn_pool in &conn_pools {
                let conn = &mut conn_pool.get().unwrap();
                assert_eq!(book::table.count().get_result::<i64>(conn).unwrap(), 0);
            }
        }
    }

    #[test]
    fn pool_drops_created_restricted_databases() {
        let backend = create_backend(false);
        test_pool_drops_created_restricted_databases(backend);
    }

    #[test]
    fn pool_drops_created_unrestricted_database() {
        let backend = create_backend(false);
        test_pool_drops_created_unrestricted_database(backend);
    }
}
