fn main() {}

#[cfg(test)]
mod tests {
    #![allow(dead_code, unused_variables)]

    use std::sync::OnceLock;

    use db_pool::{
        sync::{
            // import database pool
            DatabasePool,
            // import database pool builder trait
            DatabasePoolBuilderTrait,
            DieselPostgresBackend,
        },
        PrivilegedPostgresConfig,
    };
    use diesel::{sql_query, RunQueryDsl};
    use dotenvy::dotenv;
    use r2d2::Pool;

    fn get_connection_pool() {
        // change OnceLock inner type
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

            // create database pool
            backend.create_database_pool().unwrap()
        });
    }
}
