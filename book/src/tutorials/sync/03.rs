fn main() {}

#[cfg(test)]
mod tests {
    #![allow(dead_code, unused_variables)]

    use std::sync::OnceLock;

    use db_pool::{
        // import backend
        sync::DieselPostgresBackend,
        PrivilegedPostgresConfig,
    };
    // import diesel-specific constructs
    use diesel::{sql_query, RunQueryDsl};
    use dotenvy::dotenv;
    // import connection pool
    use r2d2::Pool;

    fn get_connection_pool() {
        static POOL: OnceLock<()> = OnceLock::new();

        let db_pool = POOL.get_or_init(|| {
            dotenv().ok();

            let config = PrivilegedPostgresConfig::from_env().unwrap();

            // create backend
            let backend = DieselPostgresBackend::new(
                config,
                // create privileged connection pool with max 10 connections
                || Pool::builder().max_size(10),
                // create restricted connection pool with max 2 connections
                || Pool::builder().max_size(2),
                // create entities
                move |conn| {
                    sql_query("CREATE TABLE book(id SERIAL PRIMARY KEY, title TEXT NOT NULL)")
                        .execute(conn)
                        .unwrap();
                },
            )
            .unwrap();
        });
    }
}
