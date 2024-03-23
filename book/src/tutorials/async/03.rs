fn main() {}

#[cfg(test)]
mod tests {
    #![allow(dead_code, unused_variables)]

    use db_pool::{
        // import backend
        r#async::{DieselAsyncPgBackend, DieselBb8},
        PrivilegedPostgresConfig,
    };
    // import diesel-specific constructs
    use diesel::sql_query;
    use diesel_async::RunQueryDsl;
    use dotenvy::dotenv;
    // import connection pool
    use bb8::Pool;
    use tokio::sync::OnceCell;

    async fn get_connection_pool() {
        static POOL: OnceCell<()> = OnceCell::const_new();
        let db_pool = POOL
            .get_or_init(|| async {
                dotenv().ok();

                let config = PrivilegedPostgresConfig::from_env().unwrap();

                // create backend for BB8 connection pools
                let backend = DieselAsyncPgBackend::<DieselBb8>::new(
                    config,
                    // create privileged connection pool with max 10 connections
                    || Pool::builder().max_size(10),
                    // create restricted connection pool with max 2 connections
                    || Pool::builder().max_size(2),
                    // create entities
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
            })
            .await;
    }
}
