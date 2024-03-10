#[cfg(feature = "diesel-postgres")]
pub mod diesel;
#[cfg(feature = "postgres")]
pub mod postgres;
mod r#trait;

#[cfg(feature = "diesel-postgres")]
pub use diesel::Backend as DieselPostgresBackend;
#[cfg(feature = "postgres")]
pub use postgres::Backend as PostgresBackend;
