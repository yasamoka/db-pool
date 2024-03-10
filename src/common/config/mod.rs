#[cfg(any(feature = "diesel-mysql", feature = "diesel-async-mysql"))]
pub(crate) mod mysql;
#[cfg(any(feature = "diesel-postgres", feature = "diesel-async-postgres"))]
pub(crate) mod postgres;

#[cfg(any(feature = "diesel-mysql", feature = "diesel-async-mysql"))]
pub use mysql::PrivilegedConfig as PrivilegedMySQLConfig;
#[cfg(any(feature = "diesel-postgres", feature = "diesel-async-postgres"))]
pub use postgres::PrivilegedConfig as PrivilegedPostgresConfig;