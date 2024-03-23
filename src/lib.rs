//! [![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/) [![Book Status](https://github.com/yasamoka/db-pool/workflows/Test%20&%20Deploy/badge.svg)](https://yasamoka.github.io/db-pool)
//!
//! A thread-safe database pool for running database-tied tests in parallel with:
//! - Easy setup
//! - Proper isolation
//! - Automatic creation, reuse, and cleanup
//! - Async support
//!
//! ### Databases
//!
//! - MySQL (MariaDB)
//! - PostgreSQL
//!
//! ### Backends & Pools
//!
//! #### Sync
//!
//! | Backend                                               | Pool                                      | Feature           |
//! | ----------------------------------------------------- | ----------------------------------------- | ----------------- |
//! | [diesel/mysql](struct@sync::DieselMySQLBackend)       | [r2d2](https://docs.rs/r2d2/0.8.10/r2d2/) | `diesel-mysql`    |
//! | [diesel/postgres](struct@sync::DieselPostgresBackend) | [r2d2](https://docs.rs/r2d2/0.8.10/r2d2/) | `diesel-postgres` |
//! | [mysql](struct@sync::MySQLBackend)                    | [r2d2](https://docs.rs/r2d2/0.8.10/r2d2/) | `mysql`           |
//! | [postgres](struct@sync::PostgresBackend)              | [r2d2](https://docs.rs/r2d2/0.8.10/r2d2/) | `postgres`        |
//!
//! #### Async
//!
//! | Backend                                                           | Pool                                                                                      | Features                                    |
//! | ----------------------------------------------------------------- | ----------------------------------------------------------------------------------------- | ------------------------------------------- |
//! | [diesel-async/mysql](struct@async::DieselAsyncMySQLBackend)       | [bb8](https://docs.rs/diesel-async/0.4.1/diesel_async/pooled_connection/bb8/index.html)   | `diesel-async-mysql`, `diesel-async-bb8`    |
//! | [diesel-async/mysql](struct@async::DieselAsyncMySQLBackend)       | [mobc](https://docs.rs/diesel-async/0.4.1/diesel_async/pooled_connection/mobc/index.html) | `diesel-async-mysql`, `diesel-async-mobc`   |
//! | [diesel-async/postgres](struct@async::DieselAsyncPostgresBackend) | [bb8](https://docs.rs/diesel-async/0.4.1/diesel_async/pooled_connection/bb8/index.html)   | `diesel-async-postgres`, `diesel-async-bb8` |
//! | [diesel-async/postgres](struct@async::DieselAsyncPostgresBackend) | [mobc](https://docs.rs/diesel-async/0.4.1/diesel_async/pooled_connection/mobc/index.html) | `diesel-async-postgres`, `diesel-async-bb8` |
//! | [sea-orm/sqlx-mysql](struct@async::SeaORMMySQLBackend)            | [sqlx](https://docs.rs/sqlx/0.7.4/sqlx/struct.Pool.html)                                  | `sea-orm-mysql`                             |
//! | [sea-orm/sqlx-postgres](struct@async::SeaORMPostgresBackend)      | [sqlx](https://docs.rs/sqlx/0.7.4/sqlx/struct.Pool.html)                                  | `sea-orm-postgres`                          |
//! | [sqlx/mysql](struct@async::SqlxMySQLBackend)                      | [sqlx](https://docs.rs/sqlx/0.7.4/sqlx/struct.Pool.html)                                  | `sqlx-mysql`                                |
//! | [sqlx/postgres](struct@async::SqlxPostgresBackend)                | [sqlx](https://docs.rs/sqlx/0.7.4/sqlx/struct.Pool.html)                                  | `sqlx-postgres`                             |
//! | [tokio-postgres](struct@async::TokioPostgresBackend)              | [bb8](https://docs.rs/bb8-postgres/0.8.1/bb8_postgres/)                                   | `tokio-postgres`, `tokio-postgres-bb8`      |
//! | [tokio-postgres](struct@async::TokioPostgresBackend)              | [mobc](https://docs.rs/mobc-postgres/0.8.0/mobc_postgres/)                                | `tokio-postgres`, `tokio-postgres-mobc`     |

#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/yasamoka/db-pool/main/logo.svg",
    html_logo_url = "https://raw.githubusercontent.com/yasamoka/db-pool/main/logo.svg",
    issue_tracker_base_url = "https://github.com/yasamoka/db-pool/issues"
)]
#![forbid(unsafe_code)]
#![deny(
    missing_docs,
    clippy::cargo,
    clippy::complexity,
    clippy::correctness,
    clippy::pedantic,
    clippy::perf,
    clippy::style,
    clippy::suspicious,
    clippy::unwrap_used
)]
#![allow(
    clippy::module_name_repetitions,
    clippy::module_inception,
    clippy::missing_errors_doc
)]

mod common;

/// Async backends
#[cfg(feature = "_async")]
pub mod r#async;
/// Sync backends
#[cfg(feature = "_sync")]
pub mod sync;
mod util;

#[allow(unused_imports)]
pub use common::config::*;

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::sync::OnceLock;

    use dotenvy::dotenv;
    use tokio::sync::RwLock;

    use crate::common::config::{mysql::PrivilegedMySQLConfig, postgres::PrivilegedPostgresConfig};

    #[cfg(feature = "_mysql")]
    pub static MYSQL_DROP_LOCK: RwLock<()> = RwLock::const_new(());

    #[cfg(feature = "_postgres")]
    pub static PG_DROP_LOCK: RwLock<()> = RwLock::const_new(());

    pub fn get_privileged_mysql_config() -> &'static PrivilegedMySQLConfig {
        static CONFIG: OnceLock<PrivilegedMySQLConfig> = OnceLock::new();
        CONFIG.get_or_init(|| {
            dotenv().ok();
            PrivilegedMySQLConfig::from_env().unwrap()
        })
    }

    pub fn get_privileged_postgres_config() -> &'static PrivilegedPostgresConfig {
        static CONFIG: OnceLock<PrivilegedPostgresConfig> = OnceLock::new();
        CONFIG.get_or_init(|| {
            dotenv().ok();
            PrivilegedPostgresConfig::from_env().unwrap()
        })
    }
}
