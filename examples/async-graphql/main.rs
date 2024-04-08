use std::env;

use async_graphql::{http::GraphiQLSource, Context, EmptySubscription, Object, SimpleObject};
use async_graphql_poem::GraphQL;
use bb8::{Pool, PooledConnection};
use db_pool::r#async::{DieselAsyncPostgresBackend, DieselBb8, PoolWrapper};
use diesel::{insert_into, prelude::*, table, Insertable};
use diesel_async::{
    pooled_connection::AsyncDieselConnectionManager, AsyncPgConnection, RunQueryDsl,
};
use dotenvy::dotenv;
use poem::{handler, listener::TcpListener, post, web::Html, IntoResponse, Route, Server};
use serde::Deserialize;

type Schema = async_graphql::Schema<Query, Mutation, EmptySubscription>;
type Manager = AsyncDieselConnectionManager<AsyncPgConnection>;
type Backend = DieselAsyncPostgresBackend<DieselBb8>;

table! {
    book (id) {
        id -> Int4,
        title -> Text
    }
}

#[derive(SimpleObject, Deserialize, Debug)]
struct Book {
    id: i32,
    title: String,
}

struct Query;

#[Object]
impl Query {
    async fn books(&self, ctx: &Context<'_>) -> Vec<Book> {
        #[derive(Queryable, Selectable)]
        #[diesel(table_name = book)]
        struct BookModel {
            id: i32,
            title: String,
        }

        let conn = &mut get_connection(ctx).await;

        book::table
            .select(BookModel::as_select())
            .load(conn)
            .await
            .unwrap()
            .drain(..)
            .map(|BookModel { id, title }| Book { id, title })
            .collect()
    }
}

struct Mutation;

#[Object]
impl Mutation {
    async fn add_book(&self, ctx: &Context<'_>, title: String) -> i32 {
        #[derive(Insertable)]
        #[diesel(table_name = book)]
        struct NewBook<'a> {
            title: &'a str,
        }

        let new_book = NewBook {
            title: title.as_str(),
        };

        let conn = &mut get_connection(ctx).await;

        insert_into(book::table)
            .values(&new_book)
            .returning(book::id)
            .get_result(conn)
            .await
            .unwrap()
    }
}

fn build_schema(conn_pool: PoolWrapper<Backend>) -> Schema {
    async_graphql::Schema::build(Query, Mutation, EmptySubscription)
        .data(conn_pool)
        .finish()
}

async fn build_default_connection_pool() -> Pool<Manager> {
    dotenv().ok();

    let username = env::var("POSTGRES_USERNAME").unwrap_or("postgres".to_owned());
    let password = env::var("POSTGRES_PASSWORD").ok();
    let host = env::var("POSTGRES_HOST").unwrap_or("localhost".to_owned());
    let port = env::var("POSTGRES_PORT")
        .map_or(Ok(3306u16), |port| port.parse())
        .unwrap();

    let db_name = "async-graphql-diesel-example";

    let connection_url = if let Some(password) = password {
        format!("postgres://{username}:{password}@{host}:{port}/{db_name}")
    } else {
        format!("postgres://{username}@{host}:{port}/{db_name}")
    };

    let manager = AsyncDieselConnectionManager::new(connection_url);
    Pool::builder()
        .test_on_check_out(true)
        .build(manager)
        .await
        .unwrap()
}

async fn get_connection<'a>(ctx: &'a Context<'_>) -> PooledConnection<'a, Manager> {
    let pool = ctx.data::<PoolWrapper<Backend>>().unwrap();
    pool.get().await.unwrap()
}

const GRAPHQL_ENDPOINT: &str = "/graphql";

#[handler]
async fn graphiql() -> impl IntoResponse {
    Html(GraphiQLSource::build().endpoint(GRAPHQL_ENDPOINT).finish())
}

#[tokio::main]
async fn main() {
    let conn_pool = build_default_connection_pool().await;
    let schema = build_schema(PoolWrapper::Pool(conn_pool));
    let app = Route::new().at(GRAPHQL_ENDPOINT, post(GraphQL::new(schema)).get(graphiql));
    let listener = TcpListener::bind("localhost:3000");
    Server::new(listener).run(app).await.unwrap();
}

#[cfg(test)]
mod tests {
    #![allow(clippy::needless_return)]

    use std::{collections::HashMap, sync::Arc};

    use async_graphql::{Request, Variables};
    use bb8::Pool;
    use db_pool::{
        r#async::{DatabasePool, DatabasePoolBuilderTrait, DieselAsyncPostgresBackend, DieselBb8},
        PrivilegedPostgresConfig,
    };
    use diesel_async_migrations::{embed_migrations, EmbeddedMigrations};
    use dotenvy::dotenv;
    use futures::future::join_all;
    use serde::{Deserialize, Serialize};
    use serde_json::{from_value, to_value, Value};
    use tokio::sync::OnceCell;
    use tokio_shared_rt::test;

    use crate::{build_schema, Book, PoolWrapper};

    async fn get_connection_pool() -> PoolWrapper<DieselAsyncPostgresBackend<DieselBb8>> {
        static MIGRATIONS: EmbeddedMigrations =
            embed_migrations!("examples/async-graphql/migrations");

        static DB_POOL: OnceCell<DatabasePool<DieselAsyncPostgresBackend<DieselBb8>>> =
            OnceCell::const_new();

        let db_pool = DB_POOL
            .get_or_init(|| async {
                dotenv().ok();

                let config = PrivilegedPostgresConfig::from_env().unwrap();

                let backend = DieselAsyncPostgresBackend::new(
                    config,
                    || Pool::builder().max_size(10),
                    || Pool::builder().max_size(1).test_on_check_out(true),
                    move |mut conn| {
                        Box::pin(async move {
                            MIGRATIONS
                                .run_pending_migrations(&mut conn)
                                .await
                                .expect("Database migrations must succeed");
                            conn
                        })
                    },
                )
                .await
                .unwrap();

                backend.create_database_pool().await.unwrap()
            })
            .await;

        let conn_pool = db_pool.pull().await;
        PoolWrapper::ReusablePool(conn_pool)
    }

    async fn test() {
        #[derive(Deserialize)]
        struct Data {
            books: Vec<Book>,
        }

        #[derive(Serialize)]
        struct AddBookVariables<'a> {
            title: &'a str,
        }

        #[derive(Deserialize, Debug)]
        #[serde(rename_all = "camelCase")]
        struct AddBookResult {
            add_book: i32,
        }

        const QUERY: &str = "{ books { id title } }";
        const MUTATION: &str = "mutation AddBook($title: String!) { addBook(title: $title) }";

        let conn_pool = get_connection_pool().await;

        let schema = Arc::new(build_schema(conn_pool));

        let titles = (0..3).map(|i| format!("Title {i}")).collect::<Vec<_>>();

        // Mutations

        let futures = titles
            .iter()
            .map(|title| {
                let schema = schema.clone();
                async move {
                    let variables = AddBookVariables { title };
                    let variables = to_value(variables).unwrap();
                    let variables = Variables::from_json(variables);
                    let request = Request::new(MUTATION).variables(variables);
                    let response = (*schema).execute(request).await;
                    assert_eq!(response.errors.len(), 0);
                    let value = Value::try_from(response.data).unwrap();
                    let result = from_value::<AddBookResult>(value).unwrap();
                    result.add_book
                }
            })
            .collect::<Vec<_>>();

        let ids: Vec<i32> = join_all(futures).await;

        let mut titles = titles;
        let mut title_map = ids
            .iter()
            .copied()
            .zip(titles.drain(..))
            .collect::<HashMap<_, _>>();

        // Query

        let response = (*schema).execute(QUERY).await;
        assert_eq!(response.errors.len(), 0);
        let value = Value::try_from(response.data).unwrap();
        let books = from_value::<Data>(value).unwrap().books;

        // Comparison

        books.iter().for_each(|book| {
            let expected_title = title_map.remove(&book.id).unwrap();
            assert_eq!(book.title, expected_title.as_str());
        })
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
