fn main() {}

#[cfg(test)]
mod tests {
    #![allow(clippy::needless_return)]

    use bb8::Pool;
    use db_pool::{
        r#async::{
            ConnectionPool, DatabasePool, DatabasePoolBuilderTrait, DieselAsyncPgBackend,
            DieselBb8, Reusable,
        },
        PrivilegedPostgresConfig,
    };
    use diesel::{insert_into, sql_query, table, Insertable, QueryDsl};
    use diesel_async::RunQueryDsl;
    use dotenvy::dotenv;
    use tokio::sync::OnceCell;
    use tokio_shared_rt::test;

    async fn get_connection_pool(
    ) -> Reusable<'static, ConnectionPool<DieselAsyncPgBackend<DieselBb8>>> {
        static POOL: OnceCell<DatabasePool<DieselAsyncPgBackend<DieselBb8>>> =
            OnceCell::const_new();

        let db_pool = POOL
            .get_or_init(|| async {
                dotenv().ok();

                let config = PrivilegedPostgresConfig::from_env().unwrap();

                let backend = DieselAsyncPgBackend::new(
                    config,
                    || Pool::builder().max_size(10),
                    || Pool::builder().max_size(2),
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

                backend.create_database_pool().await.unwrap()
            })
            .await;

        db_pool.pull().await
    }

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

    #[test(shared)]
    async fn test1() {
        test().await;
    }

    #[test(shared)]
    async fn test2() {
        test().await;
    }
}
