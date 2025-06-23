fn main() {}

#[cfg(test)]
mod tests {
    #![allow(clippy::needless_return)]

    use db_pool::{
        PrivilegedPostgresConfig,
        r#async::{
            DatabasePool, DatabasePoolBuilderTrait, ReusableConnectionPool, TokioPostgresBackend,
            TokioPostgresDeadpool,
        },
    };
    use deadpool::managed::Pool;
    use dotenvy::dotenv;
    use tokio::sync::OnceCell;
    use tokio_shared_rt::test;

    async fn get_connection_pool()
    -> ReusableConnectionPool<'static, TokioPostgresBackend<TokioPostgresDeadpool>> {
        static POOL: OnceCell<DatabasePool<TokioPostgresBackend<TokioPostgresDeadpool>>> =
            OnceCell::const_new();

        let db_pool = POOL
            .get_or_init(|| async {
                dotenv().ok();

                let config = PrivilegedPostgresConfig::from_env().unwrap();

                let backend = TokioPostgresBackend::new(
                    config.into(),
                    |manager| Pool::builder(manager).max_size(10),
                    |manager| Pool::builder(manager).max_size(2),
                    move |conn| {
                        Box::pin(async {
                            conn.execute(
                                "CREATE TABLE book(id SERIAL PRIMARY KEY, title TEXT NOT NULL)",
                                &[],
                            )
                            .await
                            .unwrap();

                            conn
                        })
                    },
                )
                .await
                .unwrap();

                backend.create_database_pool().await.unwrap()
            })
            .await;

        db_pool.pull_immutable().await
    }

    async fn test() {
        let conn_pool = get_connection_pool().await;
        let conn = &mut conn_pool.get().await.unwrap();

        conn.execute("INSERT INTO book (title) VALUES ($1)", &[&"Title"])
            .await
            .unwrap();

        let count = conn
            .query_one("SELECT COUNT(*) FROM book", &[])
            .await
            .unwrap()
            .get::<_, i64>(0);

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
