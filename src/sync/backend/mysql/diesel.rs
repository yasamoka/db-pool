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

use crate::{common::statement::mysql, util::get_db_name};

use super::r#trait::{impl_backend_for_mysql_backend, MySQLBackend};

type Manager = ConnectionManager<MysqlConnection>;

pub struct DieselMysqlBackend {
    host: String,
    port: u16,
    default_pool: Pool<Manager>,
    create_restricted_pool: Box<dyn Fn() -> Builder<Manager> + Send + Sync + 'static>,
    create_entities: Box<dyn Fn(&mut MysqlConnection) + Send + Sync + 'static>,
    drop_previous_databases_flag: bool,
}

impl DieselMysqlBackend {
    pub fn new(
        username: &str,
        password: &str,
        host: String,
        port: u16,
        create_default_pool: impl Fn() -> Builder<Manager>,
        create_restricted_pool: impl Fn() -> Builder<Manager> + Send + Sync + 'static,
        create_entities: impl Fn(&mut MysqlConnection) + Send + Sync + 'static,
    ) -> Result<Self, r2d2::Error> {
        let connection_url = format!("mysql://{username}:{password}@{host}:{port}");
        let manager = Manager::new(connection_url);
        let default_pool = (create_default_pool()).build(manager)?;

        Ok(Self {
            host,
            port,
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

    fn create_passwordless_database_url(&self, username: &str, db_name: &str) -> String {
        format!(
            "mysql://{}@{}:{}/{}",
            username, self.host, self.port, db_name
        )
    }
}

impl MySQLBackend for DieselMysqlBackend {
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
        let query = query.into_iter().collect::<Vec<_>>().join(";");
        self.execute(query.as_str(), conn)
    }

    fn get_host(&self) -> Cow<str> {
        self.host.as_str().into()
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
        let database_url = self.create_passwordless_database_url(db_name, db_name);
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

impl_backend_for_mysql_backend!(DieselMysqlBackend, Manager, ConnectionError, Error);
