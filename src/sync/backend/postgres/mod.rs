#[cfg(feature = "diesel-postgres")]
pub mod diesel;
#[cfg(feature = "postgres")]
pub mod postgres;
mod r#trait;

#[cfg(feature = "diesel-postgres")]
pub use diesel::DieselPostgresBackend;
#[cfg(feature = "postgres")]
pub use postgres::PostgresBackend;
