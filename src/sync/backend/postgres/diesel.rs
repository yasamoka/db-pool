use std::{borrow::Cow, collections::HashMap};

use diesel::{pg::PgConnection, prelude::*, r2d2::ConnectionManager, sql_query, RunQueryDsl};
use parking_lot::Mutex;
use r2d2::{Builder, Pool, PooledConnection};
use uuid::Uuid;

use crate::util::get_db_name;

use super::r#trait::{impl_backend_for_pg_backend, PostgresBackend};

type Manager = ConnectionManager<PgConnection>;

pub struct DieselPostgresBackend {
    username: String,
    password: String,
    host: String,
    port: u16,
    default_pool: Pool<Manager>,
    db_conns: Mutex<HashMap<Uuid, PgConnection>>,
    create_entities: Box<dyn Fn(&mut PgConnection) + Send + Sync + 'static>,
    create_pool_builder: Box<dyn Fn() -> Builder<Manager> + Send + Sync + 'static>,
    drop_previous_databases_flag: bool,
}

impl DieselPostgresBackend {
    pub fn new(
        username: String,
        password: String,
        host: String,
        port: u16,
        default_pool: Pool<Manager>,
        create_entities: impl Fn(&mut PgConnection) + Send + Sync + 'static,
        create_pool_builder: impl Fn() -> Builder<Manager> + Send + Sync + 'static,
    ) -> Self {
        Self {
            username,
            password,
            host,
            port,
            default_pool,
            db_conns: Mutex::new(HashMap::new()),
            create_entities: Box::new(create_entities),
            create_pool_builder: Box::new(create_pool_builder),
            drop_previous_databases_flag: true,
        }
    }

    pub fn drop_previous_databases(self, value: bool) -> Self {
        Self {
            drop_previous_databases_flag: value,
            ..self
        }
    }

    fn create_database_url(&self, username: &str, password: &str, db_name: &str) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            username, password, self.host, self.port, db_name
        )
    }
}

impl PostgresBackend for DieselPostgresBackend {
    type ConnectionManager = Manager;

    fn execute(&self, query: &str, conn: &mut PgConnection) {
        sql_query(query).execute(conn).unwrap();
    }

    fn batch_execute<'a>(
        &self,
        query: impl IntoIterator<Item = Cow<'a, str>>,
        conn: &mut PgConnection,
    ) {
        let query = query.into_iter().collect::<Vec<_>>().join(";");
        self.execute(query.as_str(), conn);
    }

    fn get_default_connection(&self) -> PooledConnection<Manager> {
        self.default_pool.get().unwrap()
    }

    fn establish_database_connection(&self, db_id: Uuid) -> PgConnection {
        let db_name = get_db_name(db_id);
        let database_url = self.create_database_url(
            self.username.as_str(),
            self.password.as_str(),
            db_name.as_str(),
        );
        PgConnection::establish(database_url.as_str()).unwrap()
    }

    fn put_database_connection(&self, db_id: Uuid, conn: PgConnection) {
        self.db_conns.lock().insert(db_id, conn);
    }

    fn get_database_connection(&self, db_id: Uuid) -> PgConnection {
        self.db_conns.lock().remove(&db_id).unwrap()
    }

    fn get_previous_database_names(&self, conn: &mut PgConnection) -> Vec<String> {
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
            .unwrap()
    }

    fn create_entities(&self, conn: &mut PgConnection) {
        (self.create_entities)(conn);
    }

    fn create_connection_pool(&self, db_id: Uuid) -> Pool<Self::ConnectionManager> {
        let db_name = get_db_name(db_id);
        let db_name = db_name.as_str();
        let database_url = self.create_database_url(db_name, db_name, db_name);
        let manager = ConnectionManager::<PgConnection>::new(database_url.as_str());
        (self.create_pool_builder)().build(manager).unwrap()
    }

    fn get_table_names(&self, privileged_conn: &mut PgConnection) -> Vec<String> {
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
            .load(privileged_conn)
            .unwrap()
    }

    fn get_drop_previous_databases(&self) -> bool {
        self.drop_previous_databases_flag
    }
}

impl_backend_for_pg_backend!(DieselPostgresBackend, Manager);
