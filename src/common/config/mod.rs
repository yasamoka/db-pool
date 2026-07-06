mod credentials;
/// MySQL configuration
#[cfg(any(
    test,
    feature = "diesel-mysql",
    feature = "diesel-async-mysql",
    feature = "sea-orm-mysql"
))]
pub mod mysql;
/// Postgres configuration
#[cfg(any(
    test,
    feature = "diesel-postgres",
    feature = "diesel-async-postgres",
    feature = "sea-orm-postgres"
))]
pub mod postgres;
