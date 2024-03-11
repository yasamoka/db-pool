#[cfg(feature = "diesel-async-mysql")]
mod diesel;
mod r#trait;

#[cfg(feature = "diesel-async-mysql")]
pub use diesel::DieselAsyncMySQLBackend;
