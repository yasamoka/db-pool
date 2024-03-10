#[cfg(feature = "diesel-mysql")]
mod diesel;
#[cfg(feature = "mysql")]
mod mysql;
mod r#trait;

#[cfg(feature = "diesel-mysql")]
pub use diesel::Backend as DieselMysqlBackend;
#[cfg(feature = "mysql")]
pub use mysql::Backend as MySQLBackend;
