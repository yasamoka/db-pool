fn main() {}

#[cfg(test)]
mod tests {
    use std::sync::OnceLock;

    use db_pool::{
        sync::{ConnectionPool, DatabasePool, DatabasePoolBuilderTrait, MySQLBackend, Reusable},
        PrivilegedMySQLConfig,
    };
    use dotenvy::dotenv;
    use mysql::{params, prelude::Queryable};
    use r2d2::Pool;

    fn get_connection_pool() -> Reusable<'static, ConnectionPool<MySQLBackend>> {
        static POOL: OnceLock<DatabasePool<MySQLBackend>> = OnceLock::new();

        let db_pool = POOL.get_or_init(|| {
            dotenv().ok();

            let config = PrivilegedMySQLConfig::from_env().unwrap();

            let backend = MySQLBackend::new(
                config.into(),
                || Pool::builder().max_size(10),
                || Pool::builder().max_size(2),
                move |conn| {
                    conn.query_drop(
                        "CREATE TABLE book(id SERIAL PRIMARY KEY, title TEXT NOT NULL)",
                    )
                    .unwrap();
                },
            )
            .unwrap();

            backend.create_database_pool().unwrap()
        });

        db_pool.pull()
    }

    fn test() {
        let conn_pool = get_connection_pool();
        let conn = &mut conn_pool.get().unwrap();

        conn.exec_drop(
            "INSERT INTO book (title) VALUES (:title)",
            params! { "title" => "Title" },
        )
        .unwrap();

        let count = conn
            .query_first::<i64, _>("SELECT COUNT(*) FROM book")
            .unwrap()
            .unwrap();

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
