use std::thread;

use r2d2::Pool;
use r2d2_postgres::{
    postgres::{Client, Config, NoTls},
    PostgresConnectionManager,
};

use db_pool::{ConnectionPool, DatabasePoolBuilder, PostgresBackend};

fn main() {
    let privileged_config = "host=localhost user=postgres".parse::<Config>().unwrap();

    let create_entities_stmt = r#"
        CREATE TABLE author(
            id uuid NOT NULL PRIMARY KEY DEFAULT gen_random_uuid(),
            first_name TEXT NOT NULL,
            last_name TEXT NOT NULL)
        "#
    .to_owned();

    let default_pool = Pool::new(PostgresConnectionManager::new(
        privileged_config.clone(),
        NoTls,
    ))
    .unwrap();

    let db_pool = PostgresBackend::new(
        privileged_config,
        default_pool,
        move |conn: &mut Client| {
            conn.execute(create_entities_stmt.as_str(), &[]).unwrap();
        },
        || Pool::builder().max_size(2),
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

fn run_test(conn_pool: &ConnectionPool<PostgresBackend>) {
    dbg!(conn_pool.db_name());

    let mut conn = conn_pool.get().unwrap();

    conn.execute(
        "INSERT INTO author (first_name, last_name) VALUES ($1, $2)",
        &[&"John", &"Doe"],
    )
    .unwrap();
}
