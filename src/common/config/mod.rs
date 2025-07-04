#[cfg(any(
    test,
    feature = "diesel-mysql",
    feature = "diesel-async-mysql",
    feature = "sea-orm-mysql"
))]
pub(crate) mod mysql;
#[cfg(any(
    test,
    feature = "diesel-postgres",
    feature = "diesel-async-postgres",
    feature = "sea-orm-postgres"
))]
pub(crate) mod postgres;

#[cfg(any(
    feature = "diesel-mysql",
    feature = "diesel-async-mysql",
    feature = "sea-orm-mysql"
))]
pub use mysql::PrivilegedMySQLConfig;
#[cfg(any(
    feature = "diesel-postgres",
    feature = "diesel-async-postgres",
    feature = "sea-orm-postgres"
))]
pub use postgres::PrivilegedPostgresConfig;
