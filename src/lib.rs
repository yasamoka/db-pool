//! [![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
//!
//! A thread-safe database pool for running database-tied tests in parallel with:
//! - Easy setup
//! - Proper isolation
//! - Automatic creation, reuse, and cleanup
//! - Async support
//!
//! ### Databases
//!
//! - ``MySQL`` (``MariaDB``)
//! - ``PostgreSQL``
//!
//! ### Backends
//!
//! #### Sync
//!
//! | Backend                                               | Feature         |
//! | ----------------------------------------------------- | --------------- |
//! | [diesel/mysql](struct@sync::DieselMySQLBackend)       | diesel-mysql    |
//! | [diesel/postgres](struct@sync::DieselPostgresBackend) | diesel-postgres |
//! | [mysql](struct@sync::MySQLBackend)                    | mysql           |
//! | [postgres](struct@sync::PostgresBackend)              | postgres        |
//!
//! #### Async
//!
//! | Backend                                                      | Feature               |
//! | ------------------------------------------------------------ | --------------------- |
//! | [diesel-async/mysql](struct@async::DieselAsyncMySQLBackend)  | diesel-async-mysql    |
//! | [diesel-async/postgres](struct@async::DieselAsyncPgBackend)  | diesel-async-postgres |
//! | [sea-orm/sqlx-mysql](struct@async::SeaORMMySQLBackend)       | sea-orm-mysql         |
//! | [sea-orm/sqlx-postgres](struct@async::SeaORMPostgresBackend) | sea-orm-postgres      |
//! | [sqlx/mysql](struct@async::SqlxMySQLBackend)                 | sqlx-mysql            |
//! | [sqlx/postgres](struct@async::SqlxPostgresBackend)           | sqlx-postgres         |
//! | [tokio-postgres](struct@async::TokioPostgresBackend)         | tokio-postgres        |

#![doc(
    html_favicon_url = "https://github.com/yasamoka/db-pool/raw/master/logo.svg",
    html_logo_url = "https://github.com/yasamoka/db-pool/raw/master/logo.svg",
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
