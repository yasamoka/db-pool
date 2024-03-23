We will use the Diesel async Postgres backend with [BB8](https://docs.rs/bb8/0.8.3/bb8/) in this tutorial.

The database pool has to live for the duration of the test run. We use a [`OnceCell`](https://docs.rs/tokio/1.36.0/tokio/sync/struct.OnceCell.html) to store a "lazy static".

```rust
{{#include 01.rs}}
```

We then need to create a privileged database configuration loaded from environment variables.

We create a .env file.

```bash
{{#include .env}}
```

The environment variables used are optional.

| Environment Variable | Default   |
| -------------------- | --------- |
| POSTGRES_USERNAME    | postgres  |
| POSTGRES_PASSWORD    | {blank}   |
| POSTGRES_HOST        | localhost |
| POSTGRES_PORT        | 3306      |

We load the environment variables from the `.env` file and create a privileged configuration.

```rust
{{#include 02.rs}}
```

`PrivilegedPostgresConfig` is the database connection configuration that includes username, password, host, and port. It is used for administration of the created databases - creation, cleaning, and dropping.

We then create the backend with the privileged configuration, among others.

```rust
{{#include 03.rs}}
```

`DieselPostgresBackend` is the backend that will communicate with the PostgreSQL instance using Diesel-specific constructs.

`DieselBb8` is a construct that allows the backend to work with and issue `BB8` [connection pools](https://docs.rs/bb8/0.8.3/bb8/struct.Pool.html).

`PrivilegedPostgresConfig::from_env().unwrap()` creates a privileged Postgres configuration from environment variables.

`|| Pool::builder().max_size(10)` creates a privileged connection pool with a max of 10 connections. This pool is used for administration of the created databases and relies on the privileged configuration to establish connections.

`|| Pool::builder().max_size(2)` creates a restricted connection pool meant to be used by a test. A restricted connection pool is created every time the database pool runs out of available connection pools to lend.

The last closure creates the database entities required to be available for tests. This happens every time a new restricted connection pool needs to be created - a new database is created, its entities are created, and the restricted connection pool is bound to it.

We create a database pool from the backend.

```rust
{{#include 04.rs}}
```

`DatabasePool` is the returned pool of connection pools that will be assigned to tests in isolation. The connection pools can be reused after a test has finished and no longer needs the connection pool assigned to it.

We pull a connection pool out of the database pool.

```rust
{{#include 05.rs}}
```

`ConnectionPool` is a connection pool assigned to a test.

`Reusable` is a wrapper around `ConnectionPool` that allows access to the connection pool by the assigned test and orchestrates its return to the database pool after a test has finished and no longer needs it.

We add a simple test case that inserts a row then counts the number of rows in a table.

```rust
{{#include 06.rs}}
```

The test gets a connection pool from the database pool every time it runs.

Finally, we add a couple of separate tests that call the same test case.

```rust
{{#include 07.rs}}
```

We use the [`test`](https://docs.rs/tokio-shared-rt/0.1.0/tokio_shared_rt/attr.test.html) macro from [`tokio_shared_rt`](https://docs.rs/tokio-shared-rt/0.1.0/tokio_shared_rt/) instead of [`tokio::test`](https://docs.rs/tokio/1.36.0/tokio/attr.test.html) in order to use a shared Tokio runtime for all tests. This is essential so that the static database pool is created and used by the same runtime and therefore remains valid for all tests.

We run the test cases in parallel and both pass.

```
running 2 tests
test tests::test1 ... ok
test tests::test2 ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
```
