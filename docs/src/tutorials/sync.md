<div class="warning">
To ensure that code in this tutorial continues to compile, some parts have been commented out with "////". To run any code yourself, uncomment those parts first.
</div>

We will use the Diesel Postgres backend in this tutorial.

The database pool has to live for the duration of the test run. We use a [`OnceLock`](https://doc.rust-lang.org/std/sync/struct.OnceLock.html) to store a "lazy static".

```rust
fn main() {}

//// #[cfg(test)]
//// mod tests {
    use std::sync::OnceLock;

    fn get_connection_pool() {
        // add "lazy static"
        static POOL: OnceLock<()> = OnceLock::new();
    }
//// }
```

We start the database pool initialization by creating a privileged configuration with the username `postgres` and password `postgres`.

```rust
fn main() {}

//// #[cfg(test)]
//// mod tests {
    use std::sync::OnceLock;

    // import privileged configuration
    use db_pool::PrivilegedPostgresConfig;

    fn get_connection_pool() {
        static POOL: OnceLock<()> = OnceLock::new();
        let db_pool = POOL.get_or_init(|| {
            //// create privileged configuration
            let config = PrivilegedPostgresConfig::new("postgres".to_owned())
                .password(Some("postgres".to_owned()));
        });
    }
//// }
```

`PrivilegedPostgresConfig` is the database connection configuration that includes username, password, host, and port. It is used for administration of the created databases - creation, cleaning, and dropping.

We then create the backend with the privileged configuration, among others.

```rust
fn main() {}

//// #[cfg(test)]
//// mod tests {
    use std::sync::OnceLock;

    use db_pool::{
        // import backend
        sync::DieselPostgresBackend,
        PrivilegedPostgresConfig,
    };
    // import diesel-specific constructs
    use diesel::{sql_query, RunQueryDsl};
    // import connection pool
    use r2d2::Pool;

    fn get_connection_pool() {
        static POOL: OnceLock<()> = OnceLock::new();
        let db_pool = POOL.get_or_init(|| {
            let config = PrivilegedPostgresConfig::new("postgres".to_owned())
                .password(Some("postgres".to_owned()));

            let backend = DieselPostgresBackend::new(
                config,
                // create privileged connection pool with max 10 connections
                || Pool::builder().max_size(10),
                // create restricted connection pool with max 2 connections
                || Pool::builder().max_size(2),
                // create entities
                move |conn| {
                    sql_query("CREATE TABLE book(id SERIAL PRIMARY KEY, title TEXT NOT NULL)")
                        .execute(conn)
                        .unwrap();
                },
            )
            .unwrap();
        });
    }
//// }
```

`DieselPostgresBackend` is the backend that will communicate with the PostgreSQL instance using Diesel-specific constructs.

`|| Pool::builder().max_size(10)` creates a privileged connection pool with a max of 10 connections. This pool is used for administration of the created databases and relies on the privileged configuration to establish connections.

`|| Pool::builder().max_size(2)` creates a restricted connection pool meant to be used by a test. A restricted connection pool is created every time the database pool runs out of available connection pools to lend.

The last closure creates the database entities required to be available for tests. This happens every time a new restricted connection pool needs to be created - a new database is created, its entities are created, and the restricted connection pool is bound to it.

We create a database pool from the backend.

```rust
fn main() {}

//// #[cfg(test)]
//// mod tests {
    use std::sync::OnceLock;

    use db_pool::{
        sync::{
            // import database pool
            DatabasePool,
            // import database pool builder trait
            DatabasePoolBuilderTrait,
            DieselPostgresBackend,
        },
        PrivilegedPostgresConfig,
    };
    use diesel::{sql_query, RunQueryDsl};
    use r2d2::Pool;

    fn get_connection_pool() {
        // change OnceLock inner type
        static POOL: OnceLock<DatabasePool<DieselPostgresBackend>> = OnceLock::new();
        let db_pool = POOL.get_or_init(|| {
            let config = PrivilegedPostgresConfig::new("postgres".to_owned())
                .password(Some("postgres".to_owned()));

            let backend = DieselPostgresBackend::new(
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

            // create database pool
            backend.create_database_pool().unwrap()
        });
    }
//// }
```

`DatabasePool` is the returned pool of connection pools that will be assigned to tests in isolation. The connection pools can be reused after a test has finished and no longer needs the connection pool assigned to it.

We pull a connection pool out of the database pool.

```rust
fn main() {}

//// #[cfg(test)]
//// mod tests {
    use std::sync::OnceLock;

    use db_pool::{
        sync::{
            // import connection pool
            ConnectionPool,
            DatabasePool,
            DatabasePoolBuilderTrait,
            DieselPostgresBackend,
            // import reusable object wrapper
            Reusable,
        },
        PrivilegedPostgresConfig,
    };
    use diesel::{sql_query, RunQueryDsl};
    use r2d2::Pool;

    // change return type
    fn get_connection_pool() -> Reusable<'static, ConnectionPool<DieselPostgresBackend>> {
        static POOL: OnceLock<DatabasePool<DieselPostgresBackend>> = OnceLock::new();
        let db_pool = POOL.get_or_init(|| {
            let config = PrivilegedPostgresConfig::new("postgres".to_owned())
                .password(Some("postgres".to_owned()));

            let backend = DieselPostgresBackend::new(
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

        // pull connection pool
        db_pool.pull()
    }
//// }
```

`ConnectionPool` is a connection pool assigned to a test.

`Reusable` is a wrapper around `ConnectionPool` that allows access to the connection pool by the assigned test and orchestrates its return to the database pool after a test has finished and no longer needs it.

We add a simple test case that inserts a row then counts the number of rows in a table.

```rust
fn main() {}

// #[cfg(test)]
// mod tests {
    use std::sync::OnceLock;

    use db_pool::{
        sync::{
            ConnectionPool, DatabasePool, DatabasePoolBuilderTrait, DieselPostgresBackend, Reusable,
        },
        PrivilegedPostgresConfig,
    };
    // import extra diesel-specific constructs
    use diesel::{insert_into, sql_query, table, Insertable, QueryDsl, RunQueryDsl};
    use r2d2::Pool;

    fn get_connection_pool() -> Reusable<'static, ConnectionPool<DieselPostgresBackend>> {
        static POOL: OnceLock<DatabasePool<DieselPostgresBackend>> = OnceLock::new();
        let db_pool = POOL.get_or_init(|| {
            let config = PrivilegedPostgresConfig::new("postgres".to_owned())
                .password(Some("postgres".to_owned()));

            let backend = DieselPostgresBackend::new(
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

        db_pool.pull()
    }

    // add test case
    fn test() {
        diesel::table! {
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
// }
```

The test gets a connection pool from the database pool every time it runs.

Finally, we add a couple of separate tests that call the same test case.

```rust
fn main() {}

//// #[cfg(test)]
//// mod tests {
    use std::sync::OnceLock;

    use db_pool::{
        sync::{
            ConnectionPool, DatabasePool, DatabasePoolBuilderTrait, DieselPostgresBackend, Reusable,
        },
        PrivilegedPostgresConfig,
    };
    use diesel::{insert_into, sql_query, table, Insertable, QueryDsl, RunQueryDsl};
    use r2d2::Pool;

    fn get_connection_pool() -> Reusable<'static, ConnectionPool<DieselPostgresBackend>> {
        static POOL: OnceLock<DatabasePool<DieselPostgresBackend>> = OnceLock::new();
        let db_pool = POOL.get_or_init(|| {
            let config = PrivilegedPostgresConfig::new("postgres".to_owned())
                .password(Some("postgres".to_owned()));

            let backend = DieselPostgresBackend::new(
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

        db_pool.pull()
    }

    fn test() {
        diesel::table! {
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

    // add first test
    #[test]
    fn test1() {
        test();
    }

    // add second test
    #[test]
    fn test2() {
        test();
    }
//// }
```

We run the test cases in parallel and both pass.

```
running 2 tests
test tests::test2 ... ok
test tests::test1 ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
```
