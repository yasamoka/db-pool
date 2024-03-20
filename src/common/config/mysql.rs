#[cfg(feature = "mysql")]
use r2d2_mysql::mysql::OptsBuilder;
#[cfg(feature = "sqlx-mysql")]
use sqlx::mysql::MySqlConnectOptions;

/// Privileged ``MySQL`` configuration
#[derive(Clone)]
pub struct PrivilegedMySQLConfig {
    pub(crate) username: String,
    pub(crate) password: Option<String>,
    pub(crate) host: String,
    pub(crate) port: u16,
}

impl PrivilegedMySQLConfig {
    /// Creates a new privileged ``MySQL`` configuration
    /// # Example
    /// ```
    /// # use db_pool::PrivilegedMySQLConfig;
    /// #
    /// let config = PrivilegedMySQLConfig::new("root".to_owned());
    /// ```
    /// # Defaults
    /// * Password: {empty}
    /// * Host: localhost
    /// * Port: 3306
    #[must_use]
    pub fn new(username: String) -> Self {
        Self {
            username,
            password: None,
            host: "localhost".to_owned(),
            port: 3306,
        }
    }

    /// Creates a new privileged ``MySQL`` configuration from environment variables
    /// # Environment variables
    /// - `MYSQL_USERNAME`
    /// - `MYSQL_PASSWORD`
    /// - `MYSQL_HOST`
    /// - `MYSQL_PORT`
    /// # Defaults
    /// * Password: {empty}
    /// * Host: localhost
    /// * Port: 3306
    pub fn from_env() -> Result<Self, Error> {
        use std::env;

        let username = env::var("MYSQL_USERNAME").map_err(|_| Error::MissingUsername)?;
        let password = env::var("MYSQL_PASSWORD").ok();
        let host = env::var("MYSQL_HOST").unwrap_or("localhost".to_owned());
        let port = env::var("MYSQL_PORT")
            .map_or(Ok(3306), |port| port.parse())
            .map_err(Error::InvalidPort)?;

        Ok(Self {
            username,
            password,
            host,
            port,
        })
    }

    /// Sets a new password
    /// # Example
    /// ```
    /// # use db_pool::PrivilegedMySQLConfig;
    /// #
    /// let config = PrivilegedMySQLConfig::new("root".to_owned()).password(Some("root".to_owned()));
    /// ```
    #[must_use]
    pub fn password(self, value: Option<String>) -> Self {
        Self {
            password: value,
            ..self
        }
    }

    /// Sets a new host
    /// # Example
    /// ```
    /// # use db_pool::PrivilegedMySQLConfig;
    /// #
    /// let config = PrivilegedMySQLConfig::new("root".to_owned()).host("localhost".to_owned());
    /// ```
    #[must_use]
    pub fn host(self, value: String) -> Self {
        Self {
            host: value,
            ..self
        }
    }

    /// Sets a new port
    /// # Example
    /// ```
    /// # use db_pool::PrivilegedMySQLConfig;
    /// #
    /// let config = PrivilegedMySQLConfig::new("root".to_owned()).port(3306);
    /// ```
    #[must_use]
    pub fn port(self, value: u16) -> Self {
        Self {
            port: value,
            ..self
        }
    }

    pub(crate) fn default_connection_url(&self) -> String {
        let Self {
            username,
            password,
            host,
            port,
        } = self;
        if let Some(password) = password {
            format!("mysql://{username}:{password}@{host}:{port}")
        } else {
            format!("mysql://{username}@{host}:{port}")
        }
    }

    pub(crate) fn privileged_database_connection_url(&self, db_name: &str) -> String {
        let Self {
            username,
            password,
            host,
            port,
            ..
        } = self;
        if let Some(password) = password {
            format!("mysql://{username}:{password}@{host}:{port}/{db_name}")
        } else {
            format!("mysql://{username}@{host}:{port}/{db_name}")
        }
    }

    pub(crate) fn restricted_database_connection_url(
        &self,
        username: &str,
        password: Option<&str>,
        db_name: &str,
    ) -> String {
        let Self { host, port, .. } = self;
        if let Some(password) = password {
            format!("mysql://{username}:{password}@{host}:{port}/{db_name}")
        } else {
            format!("mysql://{username}@{host}:{port}/{db_name}")
        }
    }
}

#[derive(Debug)]
pub enum Error {
    MissingUsername,
    InvalidPort(std::num::ParseIntError),
}

#[cfg(feature = "mysql")]
impl From<PrivilegedMySQLConfig> for OptsBuilder {
    fn from(value: PrivilegedMySQLConfig) -> Self {
        Self::new()
            .user(Some(value.username.clone()))
            .pass(value.password.clone())
            .ip_or_hostname(Some(value.host.clone()))
            .tcp_port(value.port)
    }
}

#[cfg(feature = "sqlx-mysql")]
impl From<PrivilegedMySQLConfig> for MySqlConnectOptions {
    fn from(value: PrivilegedMySQLConfig) -> Self {
        let PrivilegedMySQLConfig { username, password, host, port } = value;
        
        let opts = MySqlConnectOptions::new()
            .username(username.as_str())
            .host(host.as_str())
            .port(port);
        
        if let Some(password) = password {
            opts.password(password.as_str())
        } 
        else {
            opts
        }
    }
}
