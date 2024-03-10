use async_graphql::{http::GraphiQLSource, Context, EmptySubscription, Object, SimpleObject};
use async_graphql_poem::GraphQL;
use bb8::{Pool, PooledConnection};
use diesel::{insert_into, prelude::*, table, Insertable};
use diesel_async::{
    pooled_connection::AsyncDieselConnectionManager, AsyncPgConnection, RunQueryDsl,
};
use poem::{handler, listener::TcpListener, post, web::Html, IntoResponse, Route, Server};
use serde::Deserialize;

use db_pool::r#async::{DieselAsyncPgBackend, PoolWrapper};

type Schema = async_graphql::Schema<Query, Mutation, EmptySubscription>;
type Manager = AsyncDieselConnectionManager<AsyncPgConnection>;

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

fn build_schema(conn_pool: PoolWrapper<DieselAsyncPgBackend>) -> Schema {
    async_graphql::Schema::build(Query, Mutation, EmptySubscription)
        .data(conn_pool)
        .finish()
}

async fn build_default_connection_pool() -> Pool<Manager> {
    let manager = AsyncDieselConnectionManager::new(
        "postgres://postgres:postgres@localhost/async-graphql-diesel-example",
    );
    Pool::builder()
        .test_on_check_out(true)
        .build(manager)
        .await
        .unwrap()
}

async fn get_connection<'a>(ctx: &'a Context<'_>) -> PooledConnection<'a, Manager> {
    let pool = ctx.data::<PoolWrapper<DieselAsyncPgBackend>>().unwrap();
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
    use std::{collections::HashMap, sync::Arc};

    use async_graphql::{Request, Variables};
    use bb8::Pool;
    use diesel::sql_query;
    use futures::future::join_all;
    use serde::{Deserialize, Serialize};
    use serde_json::{from_value, to_value, Value};
    use tokio::sync::OnceCell;

    use db_pool::{
        r#async::{
            ConnectionPool, DatabasePool, DatabasePoolBuilderTrait, DieselAsyncPgBackend, Reusable,
        },
        PrivilegedPostgresConfig,
    };

    use crate::{build_default_connection_pool, build_schema, Book, PoolWrapper};

    async fn init_db_pool() -> DatabasePool<DieselAsyncPgBackend> {
        use diesel_async::RunQueryDsl;

        let backend = DieselAsyncPgBackend::new(
            PrivilegedPostgresConfig::new("postgres".to_owned())
                .password(Some("postgres".to_owned())),
            || Pool::builder().max_size(10),
            || Pool::builder().max_size(1).test_on_check_out(true),
            move |mut conn| {
                Box::pin(async move {
                    sql_query("CREATE TABLE book(id SERIAL PRIMARY KEY, title TEXT NOT NULL)")
                        .execute(&mut conn)
                        .await
                        .unwrap();
                    conn
                })
            },
        )
        .await
        .expect("backend creation must succeed");

        backend
            .create_database_pool()
            .await
            .expect("database pool creation must succeed")
    }

    async fn get_connection_pool() -> Reusable<'static, ConnectionPool<DieselAsyncPgBackend>> {
        static DB_POOL: OnceCell<DatabasePool<DieselAsyncPgBackend>> = OnceCell::const_new();
        let db_pool = DB_POOL.get_or_init(init_db_pool).await;
        db_pool.pull().await
    }

    async fn test(conn_pool: PoolWrapper<DieselAsyncPgBackend>) {
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
            .map(|id| *id)
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

    async fn run_failing_test() {
        let conn_pool = build_default_connection_pool().await;
        test(PoolWrapper::Pool(conn_pool)).await;
    }

    async fn run_passing_test() {
        for _ in 0..5 {
            let futures = (0..5)
                .map(|_| async move {
                    let conn_pool = get_connection_pool().await;
                    test(PoolWrapper::ReusablePool(conn_pool)).await;
                })
                .collect::<Vec<_>>();
            join_all(futures).await;
        }
    }

    #[ignore]
    #[tokio::test]
    async fn it_might_fail1() {
        run_failing_test().await
    }

    #[ignore]
    #[tokio::test]
    async fn it_might_fail2() {
        run_failing_test().await
    }

    #[tokio_shared_rt::test(shared)]
    async fn it_passes1() {
        run_passing_test().await
    }

    #[tokio_shared_rt::test(shared)]
    async fn it_passes2() {
        run_passing_test().await
    }
}
