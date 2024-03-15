#[cfg(feature = "diesel-async-mysql")]
mod diesel;
#[cfg(feature = "sea-orm-mysql")]
mod sea_orm;
#[cfg(feature = "sqlx-mysql")]
pub mod sqlx;
mod r#trait;

#[cfg(feature = "diesel-async-mysql")]
pub use diesel::DieselAsyncMySQLBackend;
#[cfg(feature = "sea-orm-mysql")]
pub use sea_orm::SeaORMMySQLBackend;
#[cfg(feature = "sqlx-mysql")]
pub use sqlx::SqlxMySQLBackend;
