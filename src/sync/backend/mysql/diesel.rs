use std::borrow::Cow;

use diesel::{mysql::MysqlConnection, prelude::*, r2d2::ConnectionManager, sql_query};
use r2d2::{Builder, Pool, PooledConnection};
use uuid::Uuid;

use crate::{statement::mysql, util::get_db_name};

use super::r#trait::{impl_backend_for_mysql_backend, MySQLBackend};

type Manager = ConnectionManager<MysqlConnection>;

pub struct DieselMysqlBackend {
    host: String,
    port: u16,
    default_pool: Pool<Manager>,
    create_entities: Box<dyn Fn(&mut MysqlConnection) + Send + Sync + 'static>,
    create_pool_builder: Box<dyn Fn() -> Builder<Manager> + Send + Sync + 'static>,
    terminate_connections_before_drop: bool,
}

impl DieselMysqlBackend {
    pub fn new(
        host: String,
        port: u16,
        default_pool: Pool<Manager>,
        create_entities: impl Fn(&mut MysqlConnection) + Send + Sync + 'static,
        create_pool_builder: impl Fn() -> Builder<Manager> + Send + Sync + 'static,
        terminate_connections_before_drop: bool,
    ) -> Self {
        Self {
            host,
            port,
            default_pool,
            create_entities: Box::new(create_entities),
            create_pool_builder: Box::new(create_pool_builder),
            terminate_connections_before_drop,
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

    fn get_connection(&self) -> PooledConnection<Manager> {
        self.default_pool.get().unwrap()
    }

    fn execute(&self, query: &str, conn: &mut MysqlConnection) {
        sql_query(query).execute(conn).unwrap();
    }

    fn batch_execute<'a>(
        &self,
        query: impl IntoIterator<Item = Cow<'a, str>>,
        conn: &mut MysqlConnection,
    ) {
        let query = query.into_iter().collect::<Vec<_>>().join(";");
        self.execute(query.as_str(), conn);
    }

    fn get_host(&self) -> &str {
        self.host.as_str()
    }

    fn create_entities(&self, conn: &mut MysqlConnection) {
        (self.create_entities)(conn);
    }

    fn create_connection_pool(&self, db_id: Uuid) -> Pool<Self::ConnectionManager> {
        let db_name = get_db_name(db_id);
        let db_name = db_name.as_str();
        let database_url = self.create_passwordless_database_url(db_name, db_name);
        let manager = ConnectionManager::<MysqlConnection>::new(database_url.as_str());
        (self.create_pool_builder)().build(manager).unwrap()
    }

    fn get_table_names(&self, db_name: &str, conn: &mut MysqlConnection) -> Vec<String> {
        table! {
            tables (table_name) {
                table_name -> Text,
                table_schema -> Text
            }
        }

        sql_query(mysql::USE_DEFAULT_DATABASE)
            .execute(conn)
            .unwrap();

        tables::table
            .filter(tables::table_schema.eq(db_name))
            .select(tables::table_name)
            .load::<String>(conn)
            .unwrap()
    }

    fn get_database_connection_ids(
        &self,
        db_name: &str,
        host: &str,
        conn: &mut MysqlConnection,
    ) -> Vec<i64> {
        table! {
            processlist (id) {
                id -> BigInt,
                user -> Text
            }
        }

        let user = format!("{db_name}@{host}");
        let user = user.as_str();

        processlist::table
            .filter(processlist::user.eq(user))
            .select(processlist::id)
            .load::<i64>(conn)
            .unwrap()
    }

    fn terminate_connections(&self) -> bool {
        self.terminate_connections_before_drop
    }
}

impl_backend_for_mysql_backend!(DieselMysqlBackend, Manager);
