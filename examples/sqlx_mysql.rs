use db_pool::r#async::{DatabasePoolBuilderTrait, SqlxMySQLBackend};
use futures::future::join_all;
use sqlx::{
    mysql::{MySqlConnectOptions, MySqlPoolOptions},
    query, Executor, MySqlPool,
};

#[tokio::main]
async fn main() {
    let create_stmt = r#"
        CREATE TABLE author(
            id uuid NOT NULL PRIMARY KEY DEFAULT uuid(),
            first_name TEXT NOT NULL,
            last_name TEXT NOT NULL)
        "#
    .to_owned();

    let backend = SqlxMySQLBackend::new(
        MySqlConnectOptions::new()
            .host("localhost")
            .username("postgres"),
        || MySqlPoolOptions::new().max_connections(10),
        || MySqlPoolOptions::new().max_connections(2),
        move |mut conn| {
            let create_stmt = create_stmt.clone();
            Box::pin(async move {
                conn.execute(create_stmt.as_str()).await.unwrap();
                conn
            })
        },
    );

    let db_pool = backend
        .create_database_pool()
        .await
        .expect("db_pool creation must succeed");

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

async fn run_test(conn_pool: &MySqlPool) {
    query("INSERT INTO author (first_name, last_name) VALUES (?, ?)")
        .bind("John")
        .bind("Doe")
        .execute(conn_pool)
        .await
        .unwrap();
}
