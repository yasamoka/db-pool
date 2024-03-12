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
#[cfg(feature = "diesel-async-mobc")]
pub use common::pool::diesel::mobc::DieselMobc;
#[cfg(feature = "diesel-async-mysql")]
pub use mysql::DieselAsyncMySQLBackend;
#[cfg(feature = "diesel-async-postgres")]
pub use postgres::DieselAsyncPgBackend;
#[cfg(feature = "sqlx-postgres")]
pub use postgres::SqlxPostgresBackend;
#[cfg(feature = "tokio-postgres")]
pub use postgres::TokioPostgresBackend;
pub use r#trait::Backend as AsyncBackendTrait;
