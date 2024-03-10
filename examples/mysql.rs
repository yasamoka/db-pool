use std::thread;

use r2d2::Pool;
use r2d2_mysql::mysql::{params, prelude::*, OptsBuilder};

use db_pool::{ConnectionPool, DatabasePoolBuilder, MySQLBackend};

fn main() {
    let create_entities_stmt = r#"
        CREATE TABLE author(
            id uuid NOT NULL PRIMARY KEY DEFAULT uuid(),
            first_name TEXT NOT NULL,
            last_name TEXT NOT NULL)
        "#
    .to_owned();

    let backend = MySQLBackend::new(
        OptsBuilder::new()
            .user(Some("root"))
            .pass(Some("root"))
            .into(),
        || Pool::builder().max_size(10),
        || Pool::builder().max_size(2),
        move |conn| {
            conn.query_drop(create_entities_stmt.as_str()).unwrap();
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

fn run_test(conn_pool: &ConnectionPool<MySQLBackend>) {
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
