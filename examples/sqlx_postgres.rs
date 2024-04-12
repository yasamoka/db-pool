fn main() {}

#[cfg(test)]
mod tests {
    #![allow(clippy::needless_return)]

    use db_pool::{
        r#async::{
            DatabasePool, DatabasePoolBuilderTrait, ReusableConnectionPool, SqlxPostgresBackend,
        },
        PrivilegedPostgresConfig,
    };
    use dotenvy::dotenv;
    use sqlx::{postgres::PgPoolOptions, query, Executor, Row};
    use tokio::sync::OnceCell;
    use tokio_shared_rt::test;

    async fn get_connection_pool() -> ReusableConnectionPool<'static, SqlxPostgresBackend> {
        static POOL: OnceCell<DatabasePool<SqlxPostgresBackend>> = OnceCell::const_new();

        let db_pool = POOL
            .get_or_init(|| async {
                dotenv().ok();

                let config = PrivilegedPostgresConfig::from_env().unwrap();

                let backend = SqlxPostgresBackend::new(
                    config.into(),
                    || PgPoolOptions::new().max_connections(10),
                    || PgPoolOptions::new().max_connections(2),
                    move |mut conn| {
                        Box::pin(async {
                            conn.execute(
                                "CREATE TABLE book(id SERIAL PRIMARY KEY, title TEXT NOT NULL)",
                            )
                            .await
                            .unwrap();

                            conn
                        })
                    },
                );

                backend.create_database_pool().await.unwrap()
            })
            .await;

        db_pool.pull_immutable().await
    }

    async fn test() {
        let conn_pool = get_connection_pool().await;
        let conn_pool = &**conn_pool;

        query("INSERT INTO book (title) VALUES ($1)")
            .bind("Title")
            .execute(conn_pool)
            .await
            .unwrap();

        let count = query("SELECT COUNT(*) FROM book")
            .fetch_one(conn_pool)
            .await
            .unwrap()
            .get::<i64, _>(0);

        assert_eq!(count, 1);
    }

    #[test(shared)]
    async fn test1() {
        test().await;
    }

    #[test(shared)]
    async fn test2() {
        test().await;
    }
}
