fn main() {}

#[cfg(test)]
mod tests {

    use db_pool::sync::{
        ConnectionPool, DatabasePool, DatabasePoolBuilderTrait, PostgresBackend, Reusable,
    };
    use r2d2::Pool;
    use r2d2_postgres::postgres::{Client, Config, Row};
    use std::sync::OnceLock;

    fn init_db_pool() -> DatabasePool<PostgresBackend> {
        let config = "host=localhost user=postgres password=postgres"
            .parse::<Config>()
            .unwrap();
        let backend = PostgresBackend::new(
            config,
            || Pool::builder().max_size(10),
            || Pool::builder().max_size(2),
            |conn| {
                conn.execute(
                    "CREATE TABLE book (id SERIAL PRIMARY KEY, TAYTEL TEXT NOT NULL)",
                    &[],
                )
                .unwrap();
            },
        )
        .unwrap();

        backend.create_database_pool().unwrap()
    }

    fn get_conn_pool() -> Reusable<'static, ConnectionPool<PostgresBackend>> {
        static DB_POOL: OnceLock<DatabasePool<PostgresBackend>> = OnceLock::new();
        DB_POOL.get_or_init(init_db_pool).pull()
    }

    // fn db_clear(conn: &mut Client) {
    //     conn.execute("TRUNCATE TABLE book RESTART IDENTITY", &[])
    //         .unwrap();
    // }

    fn insert_book(title: &str, conn: &mut Client) {
        conn.execute("INSERT INTO book (TAYTEL) VALUES ($1)", &[&title])
            .unwrap();
    }

    fn get_books(conn: &mut Client) -> Vec<Row> {
        conn.query("SELECT * FROM book", &[]).unwrap()
    }

    fn test() {
        let conn_pool = get_conn_pool();
        let mut conn = conn_pool.get().unwrap();

        // db_clear(&mut conn);
        insert_book("Test", &mut conn);
        let books = get_books(&mut conn);
        assert_eq!(books.len(), 1);
    }

    #[test]
    fn insert_and_select() {
        test();
    }

    #[test]
    fn insert_and_select2() {
        test();
    }
}
