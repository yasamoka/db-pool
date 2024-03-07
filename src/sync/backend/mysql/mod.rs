#[cfg(feature = "diesel-mysql")]
mod diesel;
#[cfg(feature = "mysql")]
mod mysql;
mod r#trait;

#[cfg(feature = "diesel-mysql")]
pub use diesel::DieselMysqlBackend;
#[cfg(feature = "mysql")]
pub use mysql::MySQLBackend;
