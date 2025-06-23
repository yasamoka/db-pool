mod common;
mod error;
#[cfg(feature = "_async-mysql")]
mod mysql;
#[cfg(feature = "_async-postgres")]
mod postgres;
pub(crate) mod r#trait;

pub(crate) use error::Error;

#[cfg(feature = "diesel-async-bb8")]
pub use common::pool::diesel::bb8::DieselBb8;
#[cfg(feature = "diesel-async-deadpool")]
pub use common::pool::diesel::deadpool::DieselDeadpool;
#[cfg(feature = "diesel-async-mobc")]
pub use common::pool::diesel::mobc::DieselMobc;
#[cfg(feature = "tokio-postgres-bb8")]
pub use common::pool::tokio_postgres::bb8::TokioPostgresBb8;
// #[cfg(feature = "tokio-postgres-deadpool")]
// pub use common::pool::tokio_postgres::deadpool::TokioPostgresDeadpool;
#[cfg(feature = "tokio-postgres-mobc")]
pub use common::pool::tokio_postgres::mobc::TokioPostgresMobc;
#[cfg(feature = "diesel-async-mysql")]
pub use mysql::DieselAsyncMySQLBackend;
#[cfg(feature = "sea-orm-mysql")]
pub use mysql::SeaORMMySQLBackend;
#[cfg(feature = "sqlx-mysql")]
pub use mysql::SqlxMySQLBackend;
#[cfg(feature = "diesel-async-postgres")]
pub use postgres::DieselAsyncPostgresBackend;
#[cfg(feature = "sea-orm-postgres")]
pub use postgres::SeaORMPostgresBackend;
#[cfg(feature = "sqlx-postgres")]
pub use postgres::SqlxPostgresBackend;
#[cfg(feature = "tokio-postgres")]
pub use postgres::TokioPostgresBackend;
pub use r#trait::Backend as BackendTrait;
