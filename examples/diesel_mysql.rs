use std::thread;

use diesel::{prelude::*, sql_query};
use r2d2::Pool;

use db_pool::sync::{ConnectionPool, DatabasePoolBuilderTrait, DieselMysqlBackend};

diesel::table! {
    author (id) {
        id -> Uuid,
        first_name -> Text,
        last_name -> Text,
    }
}

fn main() {
    let create_entities_stmt = r#"
        CREATE TABLE author(
            id uuid NOT NULL PRIMARY KEY DEFAULT uuid(),
            first_name TEXT NOT NULL,
            last_name TEXT NOT NULL)
        "#
    .to_owned();

    let backend = DieselMysqlBackend::new(
        "root",
        "root",
        "localhost".to_owned(),
        3306,
        || Pool::builder().max_size(10),
        || Pool::builder().max_size(2),
        move |conn| {
            sql_query(create_entities_stmt.as_str())
                .execute(conn)
                .unwrap();
        },
    )
    .expect("backend creation must succeed");

    let db_pool = backend
        .create_database_pool()
        .expect("db_pool creation must succeed");

    {
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
}

fn run_test(conn_pool: &ConnectionPool<DieselMysqlBackend>) {
    #[derive(Insertable)]
    #[diesel(table_name = author)]
    struct NewAuthor<'a> {
        first_name: &'a str,
        last_name: &'a str,
    }

    dbg!(conn_pool.db_name());

    let mut conn = conn_pool.get().unwrap();
    let new_author = NewAuthor {
        first_name: "John",
        last_name: "Doe",
    };
    diesel::insert_into(author::table)
        .values(&new_author)
        .execute(&mut conn)
        .unwrap();
}
