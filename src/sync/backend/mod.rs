mod common;
mod error;
#[cfg(feature = "_sync-mysql")]
mod mysql;
#[cfg(feature = "_sync-postgres")]
mod postgres;
pub(crate) mod r#trait;

pub use error::Error;
#[cfg(feature = "diesel-mysql")]
pub use mysql::DieselMySQLBackend;
#[cfg(feature = "mysql")]
pub use mysql::MySQLBackend;
#[cfg(feature = "diesel-postgres")]
pub use postgres::DieselPostgresBackend;
#[cfg(feature = "postgres")]
pub use postgres::PostgresBackend;
pub use r#trait::Backend as BackendTrait;
