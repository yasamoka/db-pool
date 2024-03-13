#[cfg(feature = "diesel-async-mysql")]
mod diesel;
#[cfg(feature = "sqlx-mysql")]
pub mod sqlx;
mod r#trait;

#[cfg(feature = "diesel-async-mysql")]
pub use diesel::DieselAsyncMySQLBackend;
#[cfg(feature = "sqlx-mysql")]
pub use sqlx::SqlxMySQLBackend;
