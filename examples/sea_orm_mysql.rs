use futures::future::join_all;
use sea_orm::{prelude::*, Set};

use db_pool::{
    r#async::{
        ConnectionPool, DatabasePool, DatabasePoolBuilderTrait, Reusable, SeaORMMySQLBackend,
    },
    PrivilegedMySQLConfig,
};

#[tokio::main]
async fn main() {
    let db_pool = create_database_pool().await;

    {
        for run in 0..2 {
            dbg!(run);

            let futures = (0..10)
                .map(|_| {
                    let db_pool = db_pool.clone();
                    async move {
                        let conn_pool = db_pool.pull().await;
                        run_test(conn_pool).await;
                    }
                })
                .collect::<Vec<_>>();

            join_all(futures).await;
        }
    }
}

async fn create_database_pool() -> DatabasePool<SeaORMMySQLBackend> {
    let create_stmt = r#"
        CREATE TABLE author(
            id INTEGER NOT NULL PRIMARY KEY AUTO_INCREMENT,
            first_name TEXT NOT NULL,
            last_name TEXT NOT NULL)
        "#
    .to_owned();

    let backend = SeaORMMySQLBackend::new(
        PrivilegedMySQLConfig::from_env().unwrap(),
        |opts| {
            opts.max_connections(10);
        },
        |opts| {
            opts.max_connections(2);
        },
        move |conn| {
            let create_stmt = create_stmt.clone();
            Box::pin(async move {
                conn.execute_unprepared(create_stmt.as_str()).await.unwrap();
            })
        },
    )
    .await
    .expect("backend creation must succeeed");

    backend
        .create_database_pool()
        .await
        .expect("db_pool creation must succeed")
}

async fn run_test(conn: Reusable<'_, ConnectionPool<SeaORMMySQLBackend>>) {
    #[derive(Clone, Debug, DeriveEntityModel)]
    #[sea_orm(table_name = "author")]
    pub struct Model {
        #[sea_orm(primary_key)]
        // did not use UUID due to error probably related to https://github.com/SeaQL/sea-orm/discussions/2129
        id: i32,
        first_name: String,
        last_name: String,
    }

    #[derive(Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}

    let author = ActiveModel {
        first_name: Set("John".to_owned()),
        last_name: Set("Doe".to_owned()),
        ..Default::default()
    };

    author.insert(&(**conn)).await.unwrap();
}
