use std::thread;

use diesel::{prelude::*, r2d2::ConnectionManager, sql_query};
use r2d2::Pool;

use db_pool::{
    sync::{DatabasePoolBuilderTrait, DieselPostgresBackend},
    PrivilegedPostgresConfig,
};

fn main() {
    let create_stmt = r#"
        CREATE TABLE author(
            id uuid NOT NULL PRIMARY KEY DEFAULT gen_random_uuid(),
            first_name TEXT NOT NULL,
            last_name TEXT NOT NULL)
        "#
    .to_owned();

    let backend = DieselPostgresBackend::new(
        PrivilegedPostgresConfig::new("postgres".to_owned()).password(Some("postgres".to_owned())),
        || Pool::builder().max_size(10),
        || Pool::builder().max_size(2),
        move |conn| {
            sql_query(create_stmt.as_str()).execute(conn).unwrap();
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

fn run_test(conn_pool: &Pool<ConnectionManager<PgConnection>>) {
    diesel::table! {
        author (id) {
            id -> Uuid,
            first_name -> Text,
            last_name -> Text,
        }
    }

    #[derive(Insertable)]
    #[diesel(table_name = author)]
    struct NewAuthor<'a> {
        first_name: &'a str,
        last_name: &'a str,
    }

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
