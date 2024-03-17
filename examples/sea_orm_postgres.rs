use futures::future::join_all;
use sea_orm::{prelude::*, Set};

use db_pool::{
    r#async::{
        ConnectionPool, DatabasePool, DatabasePoolBuilderTrait, Reusable, SeaORMPostgresBackend,
    },
    PrivilegedPostgresConfig,
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

async fn create_database_pool() -> DatabasePool<SeaORMPostgresBackend> {
    let create_stmt = r#"
        CREATE TABLE author(
            id uuid NOT NULL PRIMARY KEY DEFAULT gen_random_uuid(),
            first_name TEXT NOT NULL,
            last_name TEXT NOT NULL)
        "#
    .to_owned();

    let backend = SeaORMPostgresBackend::new(
        PrivilegedPostgresConfig::new("postgres".to_owned()).password(Some("postgres".to_owned())),
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

async fn run_test(conn: Reusable<'_, ConnectionPool<SeaORMPostgresBackend>>) {
    #[derive(Clone, Debug, DeriveEntityModel)]
    #[sea_orm(table_name = "author")]
    pub struct Model {
        #[sea_orm(primary_key)]
        id: Uuid,
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

    author.insert(&**conn).await.unwrap();
}
