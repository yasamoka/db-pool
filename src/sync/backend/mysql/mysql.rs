use std::borrow::Cow;

use r2d2::{Builder, Pool, PooledConnection};
use r2d2_mysql::{
    mysql::{prelude::*, Conn, Error, Opts, OptsBuilder},
    MySqlConnectionManager,
};
use uuid::Uuid;

use crate::{common::statement::mysql, util::get_db_name};

use super::{
    super::{error::Error as BackendError, r#trait::Backend},
    r#trait::{MySQLBackend as MySQLBackendTrait, MySQLBackendWrapper},
};

type Manager = MySqlConnectionManager;

/// ``MySQL`` backend
pub struct MySQLBackend {
    opts: Opts,
    default_pool: Pool<Manager>,
    create_restricted_pool: Box<dyn Fn() -> Builder<Manager> + Send + Sync + 'static>,
    create_entities: Box<dyn Fn(&mut Conn) + Send + Sync + 'static>,
    drop_previous_databases_flag: bool,
}

impl MySQLBackend {
    /// Creates a new ``MySQL`` backend
    /// # Example
    /// ```
    /// use db_pool::{sync::MySQLBackend, PrivilegedMySQLConfig};
    /// use dotenvy::dotenv;
    /// use r2d2::Pool;
    /// use r2d2_mysql::mysql::{prelude::Queryable, OptsBuilder};
    ///
    /// dotenv().ok();
    ///
    /// let config = PrivilegedMySQLConfig::from_env().unwrap();
    ///
    /// let backend = MySQLBackend::new(
    ///     config.into(),
    ///     || Pool::builder().max_size(10),
    ///     || Pool::builder().max_size(2),
    ///     move |conn| {
    ///         conn.query_drop(
    ///             "CREATE TABLE book(id INTEGER PRIMARY KEY AUTO_INCREMENT, title TEXT NOT NULL)",
    ///         )
    ///         .unwrap();
    ///     },
    /// )
    /// .unwrap();
    /// ```
    pub fn new(
        opts: Opts,
        create_privileged_pool: impl Fn() -> Builder<Manager>,
        create_restricted_pool: impl Fn() -> Builder<Manager> + Send + Sync + 'static,
        create_entities: impl Fn(&mut Conn) + Send + Sync + 'static,
    ) -> Result<Self, r2d2::Error> {
        let manager = Manager::new(OptsBuilder::from_opts(opts.clone()));
        let default_pool = (create_privileged_pool()).build(manager)?;

        Ok(Self {
            opts,
            default_pool,
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

impl MySQLBackendTrait for MySQLBackend {
    type ConnectionManager = Manager;
    type ConnectionError = Error;
    type QueryError = Error;

    fn get_connection(&self) -> Result<PooledConnection<Manager>, r2d2::Error> {
        self.default_pool.get()
    }

    fn execute(&self, query: &str, conn: &mut Conn) -> Result<(), Error> {
        conn.query_drop(query)
    }

    fn batch_execute<'a>(
        &self,
        query: impl IntoIterator<Item = Cow<'a, str>>,
        conn: &mut Conn,
    ) -> Result<(), Error> {
        let chunks = query.into_iter().collect::<Vec<_>>();
        if chunks.is_empty() {
            Ok(())
        } else {
            let query = chunks.join(";");
            self.execute(query.as_str(), conn)
        }
    }

    fn get_host(&self) -> Cow<str> {
        self.opts.get_ip_or_hostname()
    }

    fn get_previous_database_names(
        &self,
        conn: &mut <Self::ConnectionManager as r2d2::ManageConnection>::Connection,
    ) -> Result<Vec<String>, Error> {
        conn.query(mysql::GET_DATABASE_NAMES)
    }

    fn create_entities(&self, conn: &mut Conn) {
        (self.create_entities)(conn);
    }

    fn create_connection_pool(&self, db_id: Uuid) -> Result<Pool<Manager>, r2d2::Error> {
        let db_name = get_db_name(db_id);
        let db_name = db_name.as_str();
        let opts = OptsBuilder::from_opts(self.opts.clone())
            .db_name(Some(db_name))
            .user(Some(db_name))
            .pass(Some(db_name));
        let manager = MySqlConnectionManager::new(opts);
        (self.create_restricted_pool)().build(manager)
    }

    fn get_table_names(&self, db_name: &str, conn: &mut Conn) -> Result<Vec<String>, Error> {
        conn.query(mysql::get_table_names(db_name))
    }

    fn get_drop_previous_databases(&self) -> bool {
        self.drop_previous_databases_flag
    }
}

impl From<Error> for BackendError<Error, Error> {
    fn from(value: Error) -> Self {
        Self::Query(value)
    }
}

impl Backend for MySQLBackend {
    type ConnectionManager = Manager;
    type ConnectionError = Error;
    type QueryError = Error;

    fn init(&self) -> Result<(), BackendError<Error, Error>> {
        MySQLBackendWrapper::new(self).init()
    }

    fn create(&self, db_id: Uuid) -> Result<Pool<Manager>, BackendError<Error, Error>> {
        MySQLBackendWrapper::new(self).create(db_id)
    }

    fn clean(&self, db_id: Uuid) -> Result<(), BackendError<Error, Error>> {
        MySQLBackendWrapper::new(self).clean(db_id)
    }

    fn drop(&self, db_id: Uuid) -> Result<(), BackendError<Error, Error>> {
        MySQLBackendWrapper::new(self).drop(db_id)
    }
}

#[cfg(test)]
mod tests {
    #![allow(unused_variables, clippy::unwrap_used)]

    use r2d2::Pool;
    use r2d2_mysql::mysql::{params, prelude::Queryable};

    use crate::{
        common::statement::mysql::tests::{
            CREATE_ENTITIES_STATEMENT, DDL_STATEMENTS, DML_STATEMENTS,
        },
        sync::DatabasePoolBuilderTrait,
        tests::get_privileged_mysql_config,
    };

    use super::{
        super::r#trait::tests::{
            lock_read, test_backend_cleans_database_with_tables,
            test_backend_cleans_database_without_tables,
            test_backend_creates_database_with_restricted_privileges, test_backend_drops_database,
            test_backend_drops_previous_databases, test_pool_drops_created_databases,
            test_pool_drops_previous_databases,
        },
        MySQLBackend,
    };

    fn create_backend(with_table: bool) -> MySQLBackend {
        let config = get_privileged_mysql_config().clone();
        MySQLBackend::new(config.into(), Pool::builder, Pool::builder, {
            move |conn| {
                if with_table {
                    conn.query_drop(CREATE_ENTITIES_STATEMENT).unwrap();
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
            conn.exec_drop(
                "INSERT INTO book (title) VALUES (:title)",
                params! {
                    "title" => format!("Title {i}"),
                },
            )
            .unwrap();
        });

        // rows fetched must be as inserted
        conn_pools.iter().enumerate().for_each(|(i, conn_pool)| {
            let conn = &mut conn_pool.get().unwrap();
            assert_eq!(
                conn.query::<String, _>("SELECT title FROM book").unwrap(),
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

        // restricted operations
        {
            // DDL statements must fail
            for stmt in DDL_STATEMENTS {
                assert!(conn.query_drop(stmt).is_err());
            }

            // DML statements must succeed
            for stmt in DML_STATEMENTS {
                assert!(conn.query_drop(stmt).is_ok());
            }
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
                    conn.query_first::<i64, _>("SELECT COUNT(*) FROM book")
                        .unwrap()
                        .unwrap(),
                    0
                );
            }

            // insert data into each database
            for conn_pool in &conn_pools {
                let conn = &mut conn_pool.get().unwrap();
                conn.exec_drop(
                    "INSERT INTO book (title) VALUES (:title)",
                    params! {
                        "title" => "Title",
                    },
                )
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
                    conn.query_first::<i64, _>("SELECT COUNT(*) FROM book")
                        .unwrap()
                        .unwrap(),
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
