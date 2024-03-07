use bb8::Pool;
use diesel::{prelude::*, sql_query};
use diesel_async::{pooled_connection::AsyncDieselConnectionManager, RunQueryDsl};

use db_pool::{
    AsyncConnectionPool, AsyncDatabasePool, AsyncDatabasePoolBuilder, AsyncReusable,
    DieselAsyncPgBackend,
};
use futures::future::join_all;

#[tokio::main]
async fn main() {
    let db_pool = create_database_pool().await;

    {
        for run in 0..2 {
            dbg!(run);

            let futures = (0..10)
                .map(|_| {
                    let db_pool = db_pool.clone();
                    async move {
                        let conn_pool = db_pool.pull().await;
                        run_test(conn_pool).await;
                    }
                })
                .collect::<Vec<_>>();

            join_all(futures).await;
        }
    }
}

async fn create_database_pool() -> AsyncDatabasePool<DieselAsyncPgBackend> {
    let create_stmt = r#"
        CREATE TABLE author(
            id uuid NOT NULL PRIMARY KEY DEFAULT gen_random_uuid(),
            first_name TEXT NOT NULL,
            last_name TEXT NOT NULL)
        "#
    .to_owned();

    DieselAsyncPgBackend::new(
        "postgres".to_owned(),
        "postgres".to_owned(),
        "localhost".to_owned(),
        5432,
        Pool::builder()
            .build(AsyncDieselConnectionManager::new(
                "postgres://postgres:postgres@localhost:5432",
            ))
            .await
            .unwrap(),
        move |mut conn| {
            let create_stmt = create_stmt.clone();
            Box::pin(async move {
                sql_query(create_stmt.as_str())
                    .execute(&mut conn)
                    .await
                    .unwrap();
                conn
            })
        },
        || Pool::builder().max_size(2),
        false,
    )
    .create_database_pool()
}

async fn run_test(conn_pool: AsyncReusable<'_, AsyncConnectionPool<DieselAsyncPgBackend>>) {
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

    let mut conn = conn_pool.get().await.unwrap();

    let new_author = NewAuthor {
        first_name: "John",
        last_name: "Doe",
    };

    diesel::insert_into(author::table)
        .values(&new_author)
        .execute(&mut conn)
        .await
        .unwrap();
}
