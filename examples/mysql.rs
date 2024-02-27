use std::thread;

use r2d2::{Builder, Pool};
use r2d2_mysql::{
    mysql::{params, prelude::*, Conn, OptsBuilder},
    MySqlConnectionManager,
};

use db_pool::{DatabasePoolBuilder, MySQLBackend, ReusableConnectionPool};

fn main() {
    let privileged_opts = OptsBuilder::new()
        .user(Some("root"))
        .pass(Some("root"))
        .ip_or_hostname(Some("localhost"));

    let create_entities_stmt = r#"
        CREATE TABLE author(
            id uuid NOT NULL PRIMARY KEY DEFAULT uuid(),
            first_name TEXT NOT NULL,
            last_name TEXT NOT NULL)
        "#
    .to_owned();

    let default_pool = Pool::new(MySqlConnectionManager::new(privileged_opts)).unwrap();

    let db_pool = MySQLBackend::new(
        "localhost".to_owned(),
        3306,
        default_pool,
        move |conn| {
            conn.query_drop(create_entities_stmt.as_str()).unwrap();
        },
        || Pool::builder().max_size(2),
        false,
    )
    .create_database_pool();

    for run in 0..2 {
        dbg!(run);

        let mut handles = (0..10)
            .map(|_| {
                let db_pool = db_pool.clone();
                thread::spawn(move || {
                    let conn_pool = db_pool.pull();
                    run_test(&conn_pool);
                })
            })
            .collect::<Vec<_>>();

        handles.drain(..).for_each(|handle| {
            handle.join().unwrap();
        });
    }
}

fn run_test<CE, CPB>(conn_pool: &ReusableConnectionPool<MySQLBackend<CE, CPB>>)
where
    CE: Fn(&mut Conn),
    CPB: Fn() -> Builder<MySqlConnectionManager>,
{
    dbg!(conn_pool.db_name());

    let mut conn = conn_pool.get().unwrap();

    conn.exec_drop(
        "INSERT INTO author (first_name, last_name) VALUES (:first_name, :last_name)",
        params! {
            "first_name" => "John",
            "last_name" => "Doe"
        },
    )
    .unwrap();
}
