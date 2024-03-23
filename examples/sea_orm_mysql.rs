fn main() {}

#[cfg(test)]
mod tests {
    #![allow(clippy::needless_return)]

    use db_pool::{
        r#async::{
            ConnectionPool, DatabasePool, DatabasePoolBuilderTrait, Reusable, SeaORMMySQLBackend,
        },
        PrivilegedMySQLConfig,
    };
    use dotenvy::dotenv;
    use sea_orm::{prelude::*, Set};
    use tokio::sync::OnceCell;
    use tokio_shared_rt::test;

    async fn get_connection_pool() -> Reusable<'static, ConnectionPool<SeaORMMySQLBackend>> {
        static POOL: OnceCell<DatabasePool<SeaORMMySQLBackend>> = OnceCell::const_new();

        let db_pool = POOL
            .get_or_init(|| async {
                dotenv().ok();

                let config = PrivilegedMySQLConfig::from_env().unwrap();

                let backend = SeaORMMySQLBackend::new(
                    config,
                    |opts| {
                        opts.max_connections(10);
                    },
                    |opts| {
                        opts.max_connections(2);
                    },
                    move |conn| {
                        Box::pin(async move {
                            conn.execute_unprepared(
                                "CREATE TABLE book(id SERIAL PRIMARY KEY, title TEXT NOT NULL)",
                            )
                            .await
                            .unwrap();
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
        #[derive(Clone, Debug, DeriveEntityModel)]
        #[sea_orm(table_name = "book")]
        pub struct Model {
            #[sea_orm(primary_key)]
            id: u64,
            title: String,
        }

        #[derive(Debug, EnumIter, DeriveRelation)]
        pub enum Relation {}

        impl ActiveModelBehavior for ActiveModel {}

        let conn_pool = get_connection_pool().await;
        let conn = &**conn_pool;

        let book = ActiveModel {
            title: Set("Title".to_owned()),
            ..Default::default()
        };

        book.insert(conn).await.unwrap();

        let count = Entity::find().count(conn).await.unwrap();

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
