use bb8::Pool;
use bb8_postgres::{
    tokio_postgres::{Config, NoTls},
    PostgresConnectionManager,
};

use db_pool::{AsyncDatabasePoolBuilder, TokioPostgresBackend};
use futures::future::join_all;

#[tokio::main]
async fn main() {
    let privileged_config = "host=localhost user=postgres".parse::<Config>().unwrap();

    let create_stmt = r#"
        CREATE TABLE author(
            id uuid NOT NULL PRIMARY KEY DEFAULT gen_random_uuid(),
            first_name TEXT NOT NULL,
            last_name TEXT NOT NULL)
        "#
    .to_owned();

    let default_pool = Pool::builder()
        .build(PostgresConnectionManager::new(
            privileged_config.clone(),
            NoTls,
        ))
        .await
        .unwrap();

    let db_pool = TokioPostgresBackend::new(
        privileged_config,
        default_pool,
        move |conn| {
            let create_stmt = create_stmt.clone();
            Box::pin(async move {
                conn.execute(create_stmt.as_str(), &[]).await.unwrap();
                conn
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

async fn run_test(conn_pool: &Pool<PostgresConnectionManager<NoTls>>) {
    let conn = conn_pool.get().await.unwrap();

    conn.execute(
        "INSERT INTO author (first_name, last_name) VALUES ($1, $2)",
        &[&"John", &"Doe"],
    )
    .await
    .unwrap();
}
