use std::{borrow::Cow, collections::HashMap, ops::Deref};

use parking_lot::Mutex;
use r2d2::{Builder, Pool, PooledConnection};
use r2d2_postgres::{
    postgres::{Client, Config, Error, NoTls},
    PostgresConnectionManager,
};
use uuid::Uuid;

use crate::{common::statement::postgres, util::get_db_name};

use super::{
    super::{error::Error as BackendError, r#trait::Backend},
    r#trait::{PostgresBackend as PostgresBackendTrait, PostgresBackendWrapper},
};

type Manager = PostgresConnectionManager<NoTls>;

/// ``Postgres`` backend
pub struct PostgresBackend {
    config: Config,
    default_pool: Pool<Manager>,
    db_conns: Mutex<HashMap<Uuid, Client>>,
    create_restricted_pool: Box<dyn Fn() -> Builder<Manager> + Send + Sync + 'static>,
    create_entities: Box<dyn Fn(&mut Client) + Send + Sync + 'static>,
    drop_previous_databases_flag: bool,
}

impl PostgresBackend {
    /// Creates a new ``Postgres`` backend
    /// # Example
    /// ```
    /// use db_pool::{sync::PostgresBackend, PrivilegedPostgresConfig};
    /// use r2d2::Pool;
    /// use dotenvy::dotenv;
    ///
    /// dotenv().ok();
    ///
    /// let config = PrivilegedPostgresConfig::from_env().unwrap();
    ///
    /// let backend = PostgresBackend::new(
    ///     config.into(),
    ///     || Pool::builder().max_size(10),
    ///     || Pool::builder().max_size(2),
    ///     move |conn| {
    ///         conn.query(
    ///             "CREATE TABLE book(id SERIAL PRIMARY KEY, title TEXT NOT NULL)",
    ///             &[],
    ///         )
    ///         .unwrap();
    ///     },
    /// )
    /// .unwrap();
    /// ```
    pub fn new(
        config: Config,
        create_privileged_pool: impl Fn() -> Builder<Manager>,
        create_restricted_pool: impl Fn() -> Builder<Manager> + Send + Sync + 'static,
        create_entities: impl Fn(&mut Client) + Send + Sync + 'static,
    ) -> Result<Self, r2d2::Error> {
        let manager = Manager::new(config.clone(), NoTls);
        let default_pool = (create_privileged_pool()).build(manager)?;

        Ok(Self {
            config,
            default_pool,
            db_conns: Mutex::new(HashMap::new()),
            create_restricted_pool: Box::new(create_restricted_pool),
            create_entities: Box::new(create_entities),
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

impl PostgresBackendTrait for PostgresBackend {
    type ConnectionManager = Manager;
    type ConnectionError = ConnectionError;
    type QueryError = QueryError;

    fn execute(&self, query: &str, conn: &mut Client) -> Result<(), QueryError> {
        conn.execute(query, &[])?;
        Ok(())
    }

    fn batch_execute<'a>(
        &self,
        query: impl IntoIterator<Item = Cow<'a, str>>,
        conn: &mut Client,
    ) -> Result<(), QueryError> {
        let query = query.into_iter().collect::<Vec<_>>().join(";");
        conn.batch_execute(query.as_str())?;
        Ok(())
    }

    fn get_default_connection(&self) -> Result<PooledConnection<Manager>, r2d2::Error> {
        self.default_pool.get()
    }

    fn establish_database_connection(&self, db_id: Uuid) -> Result<Client, ConnectionError> {
        let mut config = self.config.clone();
        let db_name = get_db_name(db_id);
        config.dbname(db_name.as_str());
        config.connect(NoTls).map_err(Into::into)
    }

    fn put_database_connection(&self, db_id: Uuid, conn: Client) {
        self.db_conns.lock().insert(db_id, conn);
    }

    fn get_database_connection(&self, db_id: Uuid) -> Client {
        self.db_conns
            .lock()
            .remove(&db_id)
            .unwrap_or_else(|| panic!("connection map must have a connection for {db_id}"))
    }

    fn get_previous_database_names(&self, conn: &mut Client) -> Result<Vec<String>, QueryError> {
        conn.query(postgres::GET_DATABASE_NAMES, &[])
            .map(|rows| rows.iter().map(|row| row.get(0)).collect())
            .map_err(Into::into)
    }

    fn create_entities(&self, conn: &mut Client) {
        (self.create_entities)(conn);
    }

    fn create_connection_pool(&self, db_id: Uuid) -> Result<Pool<Manager>, r2d2::Error> {
        let mut config = self.config.clone();
        let db_name = get_db_name(db_id);
        let db_name = db_name.as_str();
        config.dbname(db_name);
        config.user(db_name);
        config.password(db_name);
        let manager = PostgresConnectionManager::new(config, NoTls);
        (self.create_restricted_pool)().build(manager)
    }

    fn get_table_names(&self, conn: &mut Client) -> Result<Vec<String>, QueryError> {
        conn.query(postgres::GET_TABLE_NAMES, &[])
            .map(|rows| rows.iter().map(|row| row.get(0)).collect())
            .map_err(Into::into)
    }

    fn get_drop_previous_databases(&self) -> bool {
        self.drop_previous_databases_flag
    }
}

#[derive(Debug)]
pub struct ConnectionError(Error);

impl Deref for ConnectionError {
    type Target = Error;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Error> for ConnectionError {
    fn from(value: Error) -> Self {
        Self(value)
    }
}

#[derive(Debug)]
pub struct QueryError(Error);

impl Deref for QueryError {
    type Target = Error;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Error> for QueryError {
    fn from(value: Error) -> Self {
        Self(value)
    }
}

impl From<ConnectionError> for BackendError<ConnectionError, QueryError> {
    fn from(value: ConnectionError) -> Self {
        Self::Connection(value)
    }
}

impl From<QueryError> for BackendError<ConnectionError, QueryError> {
    fn from(value: QueryError) -> Self {
        Self::Query(value)
    }
}

impl Backend for PostgresBackend {
    type ConnectionManager = Manager;
    type ConnectionError = ConnectionError;
    type QueryError = QueryError;

    fn init(&self) -> Result<(), BackendError<ConnectionError, QueryError>> {
        PostgresBackendWrapper::new(self).init()
    }

    fn create(
        &self,
        db_id: Uuid,
    ) -> Result<Pool<Manager>, BackendError<ConnectionError, QueryError>> {
        PostgresBackendWrapper::new(self).create(db_id)
    }

    fn clean(&self, db_id: Uuid) -> Result<(), BackendError<ConnectionError, QueryError>> {
        PostgresBackendWrapper::new(self).clean(db_id)
    }

    fn drop(&self, db_id: Uuid) -> Result<(), BackendError<ConnectionError, QueryError>> {
        PostgresBackendWrapper::new(self).drop(db_id)
    }
}

#[cfg(test)]
mod tests {
    #![allow(unused_variables, clippy::unwrap_used)]

    use dotenvy::dotenv;
    use r2d2::Pool;

    use crate::{
        common::statement::postgres::tests::{
            CREATE_ENTITIES_STATEMENT, DDL_STATEMENTS, DML_STATEMENTS,
        },
        sync::db_pool::DatabasePoolBuilder,
        PrivilegedPostgresConfig,
    };

    use super::{
        super::r#trait::tests::{
            lock_read, test_backend_cleans_database_with_tables,
            test_backend_cleans_database_without_tables,
            test_backend_creates_database_with_restricted_privileges, test_backend_drops_database,
            test_backend_drops_previous_databases, test_pool_drops_created_databases,
            test_pool_drops_previous_databases,
        },
        PostgresBackend,
    };

    fn create_backend(with_table: bool) -> PostgresBackend {
        dotenv().ok();

        let config = PrivilegedPostgresConfig::from_env().unwrap();

        PostgresBackend::new(config.into(), Pool::builder, Pool::builder, {
            move |conn| {
                if with_table {
                    conn.execute(CREATE_ENTITIES_STATEMENT, &[]).unwrap();
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
    fn backend_drops_database() {
        let backend = create_backend(true).drop_previous_databases(false);
        test_backend_drops_database(&backend);
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
        let conn_pools = (0..NUM_DBS).map(|_| db_pool.pull()).collect::<Vec<_>>();

        // insert single row into each database
        conn_pools.iter().enumerate().for_each(|(i, conn_pool)| {
            let conn = &mut conn_pool.get().unwrap();
            conn.execute(
                "INSERT INTO book (title) VALUES ($1)",
                &[&format!("Title {i}").as_str()],
            )
            .unwrap();
        });

        // rows fetched must be as inserted
        conn_pools.iter().enumerate().for_each(|(i, conn_pool)| {
            let conn = &mut conn_pool.get().unwrap();
            assert_eq!(
                conn.query("SELECT title FROM book", &[])
                    .unwrap()
                    .iter()
                    .map(|row| row.get::<_, String>(0))
                    .collect::<Vec<_>>(),
                vec![format!("Title {i}")]
            );
        });
    }

    #[test]
    fn pool_provides_restricted_databases() {
        let backend = create_backend(true).drop_previous_databases(false);

        let guard = lock_read();

        let db_pool = backend.create_database_pool().unwrap();

        let conn_pool = db_pool.pull();
        let conn = &mut conn_pool.get().unwrap();

        // DDL statements must fail
        for stmt in DDL_STATEMENTS {
            assert!(conn.execute(stmt, &[]).is_err());
        }

        // DML statements must succeed
        for stmt in DML_STATEMENTS {
            assert!(conn.execute(stmt, &[]).is_ok());
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
            let conn_pools = (0..NUM_DBS).map(|_| db_pool.pull()).collect::<Vec<_>>();

            // databases must be empty
            for conn_pool in &conn_pools {
                let conn = &mut conn_pool.get().unwrap();
                assert_eq!(
                    conn.query_one("SELECT COUNT(*) FROM book", &[])
                        .unwrap()
                        .get::<_, i64>(0),
                    0
                );
            }

            // insert data into each database
            for conn_pool in &conn_pools {
                let conn = &mut conn_pool.get().unwrap();
                conn.execute("INSERT INTO book (title) VALUES ($1)", &[&"Title"])
                    .unwrap();
            }
        }

        // fetch same connection pools a second time
        {
            let conn_pools = (0..NUM_DBS).map(|_| db_pool.pull()).collect::<Vec<_>>();

            // databases must be empty
            for conn_pool in &conn_pools {
                let conn = &mut conn_pool.get().unwrap();
                assert_eq!(
                    conn.query_one("SELECT COUNT(*) FROM book", &[])
                        .unwrap()
                        .get::<_, i64>(0),
                    0
                );
            }
        }
    }

    #[test]
    fn pool_drops_created_databases() {
        let backend = create_backend(false);
        test_pool_drops_created_databases(backend);
    }
}
