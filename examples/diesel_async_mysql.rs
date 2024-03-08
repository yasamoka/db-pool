use bb8::Pool;
use diesel::{prelude::*, sql_query};
use diesel_async::{
    pooled_connection::AsyncDieselConnectionManager, AsyncMysqlConnection, RunQueryDsl,
};

use db_pool::{AsyncDatabasePoolBuilder, DieselAsyncMysqlBackend};
use futures::future::join_all;

#[tokio::main]
async fn main() {
    let create_stmt = r#"
        CREATE TABLE author(
            id uuid NOT NULL PRIMARY KEY DEFAULT uuid(),
            first_name TEXT NOT NULL,
            last_name TEXT NOT NULL)
        "#
    .to_owned();

    let default_pool = Pool::builder()
        .build(AsyncDieselConnectionManager::new(
            "mysql://root:root@localhost:3306",
        ))
        .await
        .unwrap();

    let db_pool = DieselAsyncMysqlBackend::new(
        "root".to_owned(),
        "root".to_owned(),
        "localhost".to_owned(),
        3306,
        default_pool,
        move |mut conn| {
            let create_stmt = create_stmt.clone();
            Box::pin(async move {
                sql_query(create_stmt.as_str())
                    .execute(&mut conn)
                    .await
                    .unwrap();
            })
        },
        || Pool::builder().max_size(2),
        false,
    )
    .create_database_pool()
    .await;

    {
        for run in 0..2 {
            dbg!(run);

            let futures = (0..10)
                .map(|_| {
                    let db_pool = db_pool.clone();
                    async move {
                        let conn_pool = db_pool.pull().await;
                        run_test(&conn_pool).await;
                    }
                })
                .collect::<Vec<_>>();

            join_all(futures).await;
        }
    }
}

async fn run_test(conn_pool: &Pool<AsyncDieselConnectionManager<AsyncMysqlConnection>>) {
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
