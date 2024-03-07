use std::borrow::Cow;

use r2d2::{Builder, Pool, PooledConnection};
use r2d2_mysql::{
    mysql::{prelude::*, Conn, OptsBuilder},
    MySqlConnectionManager,
};
use uuid::Uuid;

use crate::{statement::mysql, util::get_db_name};

use super::r#trait::{impl_backend_for_mysql_backend, MySQLBackend as MySQLBackendTrait};

type Manager = MySqlConnectionManager;

pub struct MySQLBackend {
    host: String,
    port: u16,
    default_pool: Pool<Manager>,
    create_entities: Box<dyn Fn(&mut Conn) + Send + Sync + 'static>,
    create_pool_builder: Box<dyn Fn() -> Builder<Manager> + Send + Sync + 'static>,
    terminate_connections_before_drop: bool,
}

impl MySQLBackend {
    pub fn new(
        host: String,
        port: u16,
        default_pool: Pool<Manager>,
        create_entities: impl Fn(&mut Conn) + Send + Sync + 'static,
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
}

impl MySQLBackendTrait for MySQLBackend {
    type ConnectionManager = Manager;

    fn get_connection(&self) -> PooledConnection<Manager> {
        self.default_pool.get().unwrap()
    }

    fn execute(&self, query: &str, conn: &mut Conn) {
        conn.query_drop(query).unwrap();
    }

    fn batch_execute<'a>(&self, query: impl IntoIterator<Item = Cow<'a, str>>, conn: &mut Conn) {
        let query = query.into_iter().collect::<Vec<_>>().join(";");
        self.execute(query.as_str(), conn);
    }

    fn get_host(&self) -> &str {
        self.host.as_str()
    }

    fn create_entities(&self, conn: &mut Conn) {
        (self.create_entities)(conn);
    }

    fn create_connection_pool(&self, db_id: Uuid) -> Pool<Manager> {
        let db_name = get_db_name(db_id);
        let db_name = db_name.as_str();
        let opts = OptsBuilder::new()
            .ip_or_hostname(Some(self.host.as_str()))
            .tcp_port(self.port)
            .db_name(Some(db_name))
            .user(Some(db_name));
        let manager = MySqlConnectionManager::new(opts);
        (self.create_pool_builder)().build(manager).unwrap()
    }

    fn get_table_names(&self, db_name: &str, conn: &mut Conn) -> Vec<String> {
        conn.query(mysql::get_table_names(db_name))
            .unwrap()
            .drain(..)
            .collect()
    }

    fn get_database_connection_ids(&self, db_name: &str, host: &str, conn: &mut Conn) -> Vec<i64> {
        conn.query(mysql::get_database_connection_ids(db_name, host))
            .unwrap()
    }

    fn terminate_connections(&self) -> bool {
        self.terminate_connections_before_drop
    }
}

impl_backend_for_mysql_backend!(MySQLBackend, Manager);
