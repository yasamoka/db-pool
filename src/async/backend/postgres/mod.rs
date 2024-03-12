#[cfg(feature = "diesel-async-postgres")]
mod diesel;
#[cfg(feature = "sqlx-postgres")]
pub mod sqlx;
#[cfg(feature = "tokio-postgres")]
mod tokio;
mod r#trait;

#[cfg(feature = "diesel-async-postgres")]
pub use diesel::DieselAsyncPgBackend;
#[cfg(feature = "sqlx-postgres")]
pub use sqlx::SqlxPostgresBackend;
#[cfg(feature = "tokio-postgres")]
pub use tokio::TokioPostgresBackend;
