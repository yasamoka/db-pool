use std::thread;

use r2d2::Pool;
use r2d2_postgres::postgres::{Client, Config};

use db_pool::sync::{ConnectionPool, DatabasePoolBuilderTrait, PostgresBackend};

fn main() {
    let create_entities_stmt = r#"
        CREATE TABLE author(
            id uuid NOT NULL PRIMARY KEY DEFAULT gen_random_uuid(),
            first_name TEXT NOT NULL,
            last_name TEXT NOT NULL)
        "#
    .to_owned();

    let backend = PostgresBackend::new(
        "host=localhost user=postgres".parse::<Config>().unwrap(),
        || Pool::builder().max_size(10),
        || Pool::builder().max_size(2),
        move |conn: &mut Client| {
            conn.execute(create_entities_stmt.as_str(), &[]).unwrap();
        },
    )
    .expect("backend creation must succeed");

    let db_pool = backend
        .create_database_pool()
        .expect("db_pool creation must succeed");

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
    let mut conn = conn_pool.get().unwrap();

    conn.execute(
        "INSERT INTO author (first_name, last_name) VALUES ($1, $2)",
        &[&"John", &"Doe"],
    )
    .unwrap();
}
