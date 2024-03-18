use std::borrow::Cow;

use diesel::{
    mysql::MysqlConnection,
    prelude::*,
    r2d2::ConnectionManager,
    result::{ConnectionError, Error, QueryResult},
    sql_query,
};
use r2d2::{Builder, Pool, PooledConnection};
use uuid::Uuid;

use crate::{
    common::{config::mysql::PrivilegedConfig, statement::mysql},
    util::get_db_name,
};

use super::r#trait::{impl_backend_for_mysql_backend, MySQLBackend};

type Manager = ConnectionManager<MysqlConnection>;

pub struct DieselMySQLBackend {
    privileged_config: PrivilegedConfig,
    default_pool: Pool<Manager>,
    create_restricted_pool: Box<dyn Fn() -> Builder<Manager> + Send + Sync + 'static>,
    create_entities: Box<dyn Fn(&mut MysqlConnection) + Send + Sync + 'static>,
    drop_previous_databases_flag: bool,
}

impl DieselMySQLBackend {
    pub fn new(
        privileged_config: PrivilegedConfig,
        create_privileged_pool: impl Fn() -> Builder<Manager>,
        create_restricted_pool: impl Fn() -> Builder<Manager> + Send + Sync + 'static,
        create_entities: impl Fn(&mut MysqlConnection) + Send + Sync + 'static,
    ) -> Result<Self, r2d2::Error> {
        let manager = Manager::new(privileged_config.default_connection_url());
        let default_pool = (create_privileged_pool()).build(manager)?;

        Ok(Self {
            privileged_config,
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

impl MySQLBackend for DieselMySQLBackend {
    type ConnectionManager = Manager;
    type ConnectionError = ConnectionError;
    type QueryError = Error;

    fn get_connection(&self) -> Result<PooledConnection<Manager>, r2d2::Error> {
        self.default_pool.get()
    }

    fn execute(&self, query: &str, conn: &mut MysqlConnection) -> QueryResult<()> {
        sql_query(query).execute(conn)?;
        Ok(())
    }

    fn batch_execute<'a>(
        &self,
        query: impl IntoIterator<Item = Cow<'a, str>>,
        conn: &mut MysqlConnection,
    ) -> QueryResult<()> {
        let chunks = query.into_iter().collect::<Vec<_>>();
        if chunks.is_empty() {
            Ok(())
        } else {
            let query = chunks.join(";");
            self.execute(query.as_str(), conn)
        }
    }

    fn get_host(&self) -> Cow<str> {
        self.privileged_config.host.as_str().into()
    }

    fn get_previous_database_names(
        &self,
        conn: &mut <Self::ConnectionManager as r2d2::ManageConnection>::Connection,
    ) -> QueryResult<Vec<String>> {
        table! {
            schemata (schema_name) {
                schema_name -> Text
            }
        }

        schemata::table
            .select(schemata::schema_name)
            .filter(schemata::schema_name.like("db_pool_%"))
            .load::<String>(conn)
    }

    fn create_entities(&self, conn: &mut MysqlConnection) {
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
        let manager = ConnectionManager::<MysqlConnection>::new(database_url.as_str());
        (self.create_restricted_pool)().build(manager)
    }

    fn get_table_names(
        &self,
        db_name: &str,
        conn: &mut MysqlConnection,
    ) -> QueryResult<Vec<String>> {
        table! {
            tables (table_name) {
                table_name -> Text,
                table_schema -> Text
            }
        }

        sql_query(mysql::USE_DEFAULT_DATABASE).execute(conn)?;

        tables::table
            .filter(tables::table_schema.eq(db_name))
            .select(tables::table_name)
            .load::<String>(conn)
    }

    fn get_drop_previous_databases(&self) -> bool {
        self.drop_previous_databases_flag
    }
}

impl_backend_for_mysql_backend!(DieselMySQLBackend, Manager, ConnectionError, Error);

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use diesel::{sql_query, RunQueryDsl};
    use r2d2::Pool;

    use crate::common::{
        config::PrivilegedMySQLConfig, statement::mysql::tests::CREATE_ENTITIES_STATEMENT,
    };

    use super::{
        super::r#trait::tests::{
            test_cleans_database_with_tables, test_cleans_database_without_tables,
            test_creates_database_with_restricted_privileges, test_drops_database,
            test_drops_previous_databases,
        },
        DieselMySQLBackend,
    };

    fn create_backend(with_table: bool) -> DieselMySQLBackend {
        DieselMySQLBackend::new(
            PrivilegedMySQLConfig::new("root".to_owned()).password(Some("root".to_owned())),
            Pool::builder,
            Pool::builder,
            {
                move |conn| {
                    if with_table {
                        sql_query(CREATE_ENTITIES_STATEMENT).execute(conn).unwrap();
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
