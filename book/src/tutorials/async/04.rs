fn main() {}

#[cfg(test)]
mod tests {
    #![allow(dead_code, unused_variables)]

    use bb8::Pool;
    use db_pool::{
        r#async::{
            // import database pool
            DatabasePool,
            // import database pool builder trait
            DatabasePoolBuilderTrait,
            DieselAsyncPostgresBackend,
            DieselBb8,
        },
        PrivilegedPostgresConfig,
    };
    use diesel::sql_query;
    use diesel_async::RunQueryDsl;
    use dotenvy::dotenv;
    use tokio::sync::OnceCell;

    async fn get_connection_pool() {
        // change OnceCell inner type
        static POOL: OnceCell<DatabasePool<DieselAsyncPostgresBackend<DieselBb8>>> =
            OnceCell::const_new();

        let db_pool = POOL.get_or_init(|| async {
            dotenv().ok();

            let config = PrivilegedPostgresConfig::from_env().unwrap();

            // Diesel pool association type can be inferred now
            let backend = DieselAsyncPostgresBackend::new(
                config,
                || Pool::builder().max_size(10),
                || Pool::builder().max_size(2),
                None,
                move |mut conn| {
                    Box::pin(async {
                        sql_query("CREATE TABLE book(id SERIAL PRIMARY KEY, title TEXT NOT NULL)")
                            .execute(&mut conn)
                            .await
                            .unwrap();

                        conn
                    })
                },
            )
            .await
            .unwrap();

            // create database pool
            backend.create_database_pool().await.unwrap()
        });
    }
}
