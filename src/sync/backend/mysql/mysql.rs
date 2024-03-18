use std::borrow::Cow;

use r2d2::{Builder, Pool, PooledConnection};
use r2d2_mysql::{
    mysql::{prelude::*, Conn, Error, Opts, OptsBuilder},
    MySqlConnectionManager,
};
use uuid::Uuid;

use crate::{
    common::statement::mysql, sync::backend::error::Error as BackendError, util::get_db_name,
};

use super::r#trait::{impl_backend_for_mysql_backend, MySQLBackend as MySQLBackendTrait};

type Manager = MySqlConnectionManager;

pub struct MySQLBackend {
    opts: Opts,
    default_pool: Pool<Manager>,
    create_restricted_pool: Box<dyn Fn() -> Builder<Manager> + Send + Sync + 'static>,
    create_entities: Box<dyn Fn(&mut Conn) + Send + Sync + 'static>,
    drop_previous_databases_flag: bool,
}

impl MySQLBackend {
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

impl_backend_for_mysql_backend!(MySQLBackend, Manager, Error, Error);

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use r2d2::Pool;
    use r2d2_mysql::mysql::{prelude::Queryable, OptsBuilder};

    use crate::common::statement::mysql::tests::CREATE_ENTITIES_STATEMENT;

    use super::{
        super::r#trait::tests::{
            test_cleans_database_with_tables, test_cleans_database_without_tables,
            test_creates_database_with_restricted_privileges, test_drops_database,
            test_drops_previous_databases,
        },
        MySQLBackend,
    };

    fn create_backend(with_table: bool) -> MySQLBackend {
        MySQLBackend::new(
            OptsBuilder::new()
                .user(Some("root"))
                .pass(Some("root"))
                .into(),
            Pool::builder,
            Pool::builder,
            {
                move |conn| {
                    if with_table {
                        conn.query_drop(CREATE_ENTITIES_STATEMENT).unwrap();
                    }
                }
            },
        )
        .unwrap()
    }

    #[test]
    fn drops_previous_databases() {
        test_drops_previous_databases(
            create_backend(false),
            create_backend(false).drop_previous_databases(true),
            create_backend(false).drop_previous_databases(false),
        );
    }

    #[test]
    fn creates_database_with_restricted_privileges() {
        let backend = create_backend(true).drop_previous_databases(false);
        test_creates_database_with_restricted_privileges(&backend);
    }

    #[test]
    fn cleans_database_with_tables() {
        let backend = create_backend(true).drop_previous_databases(false);
        test_cleans_database_with_tables(&backend);
    }

    #[test]
    fn cleans_database_without_tables() {
        let backend = create_backend(false).drop_previous_databases(false);
        test_cleans_database_without_tables(&backend);
    }

    #[test]
    fn drops_database() {
        let backend = create_backend(true).drop_previous_databases(false);
        test_drops_database(&backend);
    }
}
