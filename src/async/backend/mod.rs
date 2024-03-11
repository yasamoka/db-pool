mod common;
mod error;
#[cfg(feature = "_async-mysql")]
mod mysql;
#[cfg(feature = "_async-postgres")]
mod postgres;
pub(crate) mod r#trait;

pub(crate) use error::Error;

#[cfg(feature = "diesel-async-mysql")]
pub use mysql::DieselAsyncMySQLBackend;
#[cfg(feature = "diesel-async-postgres")]
pub use postgres::DieselAsyncPostgresBackend;
#[cfg(feature = "tokio-postgres")]
pub use postgres::TokioPostgresBackend;
pub use r#trait::Backend as AsyncBackendTrait;
