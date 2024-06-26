fn main() {}

#[cfg(test)]
mod tests {
    #![allow(dead_code)]

    use std::sync::OnceLock;

    use db_pool::{
        sync::{
            DatabasePool,
            DatabasePoolBuilderTrait,
            DieselPostgresBackend,
            // import reusable connection pool
            ReusableConnectionPool,
        },
        PrivilegedPostgresConfig,
    };
    use diesel::{sql_query, RunQueryDsl};
    use dotenvy::dotenv;
    use r2d2::Pool;

    // change return type
    fn get_connection_pool() -> ReusableConnectionPool<'static, DieselPostgresBackend> {
        static POOL: OnceLock<DatabasePool<DieselPostgresBackend>> = OnceLock::new();

        let db_pool = POOL.get_or_init(|| {
            dotenv().ok();

            let config = PrivilegedPostgresConfig::from_env().unwrap();

            let backend = DieselPostgresBackend::new(
                config,
                || Pool::builder().max_size(10),
                || Pool::builder().max_size(2),
                move |conn| {
                    sql_query("CREATE TABLE book(id SERIAL PRIMARY KEY, title TEXT NOT NULL)")
                        .execute(conn)
                        .unwrap();
                },
            )
            .unwrap();

            backend.create_database_pool().unwrap()
        });

        // pull connection pool
        db_pool.pull_immutable()
    }
}
