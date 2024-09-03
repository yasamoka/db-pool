fn main() {}

#[cfg(test)]
mod tests {
    #![allow(dead_code)]

    use bb8::Pool;
    use db_pool::{
        r#async::{
            DatabasePool,
            DatabasePoolBuilderTrait,
            DieselAsyncPostgresBackend,
            DieselBb8,
            // import reusable connection pool
            ReusableConnectionPool,
        },
        PrivilegedPostgresConfig,
    };
    use diesel::sql_query;
    use diesel_async::{pooled_connection::ManagerConfig, RunQueryDsl};
    use dotenvy::dotenv;
    use tokio::sync::OnceCell;

    // change return type
    async fn get_connection_pool(
    ) -> ReusableConnectionPool<'static, DieselAsyncPostgresBackend<DieselBb8>> {
        static POOL: OnceCell<DatabasePool<DieselAsyncPostgresBackend<DieselBb8>>> =
            OnceCell::const_new();

        let db_pool = POOL
            .get_or_init(|| async {
                dotenv().ok();

                let config = PrivilegedPostgresConfig::from_env().unwrap();

                let backend = DieselAsyncPostgresBackend::new(
                    config,
                    ManagerConfig::default(),
                    || Pool::builder().max_size(10),
                    || Pool::builder().max_size(2),
                    move |mut conn| {
                        Box::pin(async {
                            sql_query(
                                "CREATE TABLE book(id SERIAL PRIMARY KEY, title TEXT NOT NULL)",
                            )
                            .execute(&mut conn)
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

        // pull connection pool
        db_pool.pull_immutable().await
    }
}
