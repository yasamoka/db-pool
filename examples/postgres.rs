fn main() {}

#[cfg(test)]
mod tests {
    use std::sync::OnceLock;

    use db_pool::{
        sync::{DatabasePool, DatabasePoolBuilderTrait, PostgresBackend, ReusableConnectionPool},
        PrivilegedPostgresConfig,
    };
    use dotenvy::dotenv;
    use r2d2::Pool;

    fn get_connection_pool() -> ReusableConnectionPool<'static, PostgresBackend> {
        static POOL: OnceLock<DatabasePool<PostgresBackend>> = OnceLock::new();

        let db_pool = POOL.get_or_init(|| {
            dotenv().ok();

            let config = PrivilegedPostgresConfig::from_env().unwrap();

            let backend = PostgresBackend::new(
                config.into(),
                || Pool::builder().max_size(10),
                || Pool::builder().max_size(2),
                move |conn| {
                    conn.execute(
                        "CREATE TABLE book(id SERIAL PRIMARY KEY, title TEXT NOT NULL)",
                        &[],
                    )
                    .unwrap();
                },
            )
            .unwrap();

            backend.create_database_pool().unwrap()
        });

        db_pool.pull_immutable()
    }

    fn test() {
        let conn_pool = get_connection_pool();
        let conn = &mut conn_pool.get().unwrap();

        conn.execute("INSERT INTO book (title) VALUES ($1)", &[&"Title"])
            .unwrap();

        let count = conn
            .query_one("SELECT COUNT(*) FROM book", &[])
            .unwrap()
            .get::<_, i64>(0);

        assert_eq!(count, 1);
    }

    #[test]
    fn test1() {
        test();
    }

    #[test]
    fn test2() {
        test();
    }
}
