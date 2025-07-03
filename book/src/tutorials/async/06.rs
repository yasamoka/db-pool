fn main() {}

#[cfg(test)]
mod tests {
    #![allow(dead_code)]

    use bb8::Pool;
    use db_pool::{
        PrivilegedPostgresConfig,
        r#async::{
            DatabasePool, DatabasePoolBuilderTrait, DieselAsyncPostgresBackend, DieselBb8,
            ReusableConnectionPool,
        },
    };
    // import extra diesel-specific constructs
    use diesel::{Insertable, QueryDsl, insert_into, sql_query, table};
    use diesel_async::RunQueryDsl;
    use dotenvy::dotenv;
    use tokio::sync::OnceCell;

    async fn get_connection_pool()
    -> ReusableConnectionPool<'static, DieselAsyncPostgresBackend<DieselBb8>> {
        static POOL: OnceCell<DatabasePool<DieselAsyncPostgresBackend<DieselBb8>>> =
            OnceCell::const_new();

        let db_pool = POOL
            .get_or_init(|| async {
                dotenv().ok();

                let config = PrivilegedPostgresConfig::from_env().unwrap();

                let backend = DieselAsyncPostgresBackend::new(
                    config,
                    |_| Pool::builder().max_size(10),
                    |_| Pool::builder().max_size(2),
                    None,
                    move |mut conn| {
                        Box::pin(async {
                            sql_query(
                                "CREATE TABLE book(id SERIAL PRIMARY KEY, title TEXT NOT NULL)",
                            )
                            .execute(&mut conn)
                            .await
                            .unwrap();

                            Some(conn)
                        })
                    },
                )
                .await
                .unwrap();

                backend.create_database_pool().await.unwrap()
            })
            .await;

        db_pool.pull_immutable().await
    }

    // add test case
    async fn test() {
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

        // get connection pool from database pool
        let conn_pool = get_connection_pool().await;
        let conn = &mut conn_pool.get().await.unwrap();

        let new_book = NewBook { title: "Title" };

        insert_into(book::table)
            .values(&new_book)
            .execute(conn)
            .await
            .unwrap();

        let count = book::table.count().get_result::<i64>(conn).await.unwrap();
        assert_eq!(count, 1);
    }
}
