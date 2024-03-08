use std::thread;

use diesel::{prelude::*, r2d2::ConnectionManager, sql_query};
use r2d2::Pool;

use db_pool::{ConnectionPool, DatabasePoolBuilder, DieselMysqlBackend};

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

    let default_pool =
        Pool::new(ConnectionManager::new("mysql://root:root@localhost:3306")).unwrap();

    let db_pool = DieselMysqlBackend::new(
        "localhost".to_owned(),
        3306,
        default_pool,
        move |conn| {
            sql_query(create_entities_stmt.as_str())
                .execute(conn)
                .unwrap();
        },
        || Pool::builder().max_size(2),
    )
    .create_database_pool();

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
