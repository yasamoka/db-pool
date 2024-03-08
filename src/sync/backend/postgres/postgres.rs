use std::{borrow::Cow, collections::HashMap};

use parking_lot::Mutex;
use r2d2::{Builder, Pool, PooledConnection};
use r2d2_postgres::{
    postgres::{Client, Config, NoTls},
    PostgresConnectionManager,
};
use uuid::Uuid;

use crate::{statement::pg, util::get_db_name};

use super::r#trait::{impl_backend_for_pg_backend, PostgresBackend as PostgresBackendTrait};

type Manager = PostgresConnectionManager<NoTls>;

pub struct PostgresBackend {
    privileged_config: Config,
    default_pool: Pool<Manager>,
    db_conns: Mutex<HashMap<Uuid, Client>>,
    create_entities: Box<dyn Fn(&mut Client) + Send + Sync + 'static>,
    create_pool_builder: Box<dyn Fn() -> Builder<Manager> + Send + Sync + 'static>,
    drop_previous_databases_flag: bool,
}

impl PostgresBackend {
    pub fn new(
        privileged_config: Config,
        default_pool: Pool<Manager>,
        create_entities: impl Fn(&mut Client) + Send + Sync + 'static,
        create_pool_builder: impl Fn() -> Builder<Manager> + Send + Sync + 'static,
    ) -> Self {
        Self {
            privileged_config,
            default_pool,
            db_conns: Mutex::new(HashMap::new()),
            create_entities: Box::new(create_entities),
            create_pool_builder: Box::new(create_pool_builder),
            drop_previous_databases_flag: true,
        }
    }

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

    fn execute(&self, query: &str, conn: &mut Client) {
        conn.execute(query, &[]).unwrap();
    }

    fn batch_execute<'a>(&self, query: impl IntoIterator<Item = Cow<'a, str>>, conn: &mut Client) {
        let query = query.into_iter().collect::<Vec<_>>().join(";");
        conn.batch_execute(query.as_str()).unwrap();
    }

    fn establish_database_connection(&self, db_id: Uuid) -> Client {
        let mut config = self.privileged_config.clone();
        let db_name = get_db_name(db_id);
        config.dbname(db_name.as_str());
        config.connect(NoTls).unwrap()
    }

    fn get_default_connection(&self) -> PooledConnection<Manager> {
        self.default_pool.get().unwrap()
    }

    fn put_database_connection(&self, db_id: Uuid, conn: Client) {
        self.db_conns.lock().insert(db_id, conn);
    }

    fn get_database_connection(&self, db_id: Uuid) -> Client {
        self.db_conns.lock().remove(&db_id).unwrap()
    }

    fn get_previous_database_names(&self, conn: &mut Client) -> Vec<String> {
        conn.query(pg::GET_DATABASE_NAMES, &[])
            .unwrap()
            .drain(..)
            .map(|row| row.get(0))
            .collect()
    }

    fn create_entities(&self, conn: &mut Client) {
        (self.create_entities)(conn);
    }

    fn create_connection_pool(&self, db_id: Uuid) -> Pool<Manager> {
        let mut config = self.privileged_config.clone();
        let db_name = get_db_name(db_id);
        let db_name = db_name.as_str();
        config.dbname(db_name);
        config.user(db_name);
        let manager = PostgresConnectionManager::new(config, NoTls);
        (self.create_pool_builder)().build(manager).unwrap()
    }

    fn get_table_names(&self, privileged_conn: &mut Client) -> Vec<String> {
        privileged_conn
            .query(pg::GET_TABLE_NAMES, &[])
            .unwrap()
            .drain(..)
            .map(|row| row.get(0))
            .collect()
    }

    fn get_drop_previous_databases(&self) -> bool {
        self.drop_previous_databases_flag
    }
}

impl_backend_for_pg_backend!(PostgresBackend, Manager);
