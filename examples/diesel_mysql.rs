fn main() {}

#[cfg(test)]
mod tests {
    use std::sync::OnceLock;

    use db_pool::{
        sync::{
            DatabasePool, DatabasePoolBuilderTrait, DieselMySQLBackend, ReusableConnectionPool,
        },
        PrivilegedMySQLConfig,
    };
    use diesel::{insert_into, sql_query, table, Insertable, QueryDsl, RunQueryDsl};
    use dotenvy::dotenv;
    use r2d2::Pool;

    fn get_connection_pool() -> ReusableConnectionPool<'static, DieselMySQLBackend> {
        static POOL: OnceLock<DatabasePool<DieselMySQLBackend>> = OnceLock::new();

        let db_pool = POOL.get_or_init(|| {
            dotenv().ok();

            let config = PrivilegedMySQLConfig::from_env().unwrap();

            let backend = DieselMySQLBackend::new(
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

        db_pool.pull_immutable()
    }

    fn test() {
        table! {
            book (id) {
                id -> Int4,
                title -> Text
            }
        }

        #[derive(Insertable)]
        #[diesel(table_name = book)]
        struct NewBook<'a> {
            title: &'a str,
        }

        let conn_pool = get_connection_pool();
        let conn = &mut conn_pool.get().unwrap();

        let new_book = NewBook { title: "Title" };

        insert_into(book::table)
            .values(&new_book)
            .execute(conn)
            .unwrap();

        let count = book::table.count().get_result::<i64>(conn).unwrap();

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
