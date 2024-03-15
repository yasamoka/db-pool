#[cfg(feature = "diesel-async-postgres")]
mod diesel;
#[cfg(feature = "sea-orm-postgres")]
mod sea_orm;
#[cfg(feature = "sqlx-postgres")]
pub mod sqlx;
#[cfg(feature = "tokio-postgres")]
mod tokio_postgres;
mod r#trait;

#[cfg(feature = "diesel-async-postgres")]
pub use diesel::DieselAsyncPgBackend;
#[cfg(feature = "sea-orm-postgres")]
pub use sea_orm::SeaORMPostgresBackend;
#[cfg(feature = "sqlx-postgres")]
pub use sqlx::SqlxPostgresBackend;
#[cfg(feature = "tokio-postgres")]
pub use tokio_postgres::TokioPostgresBackend;
