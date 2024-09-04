/// Privileged MySQL configuration
#[derive(Clone)]
pub struct PrivilegedMySQLConfig {
    pub(crate) username: String,
    pub(crate) password: Option<String>,
    pub(crate) host: String,
    pub(crate) port: u16,
}

impl PrivilegedMySQLConfig {
    const DEFAULT_USERNAME: &'static str = "root";
    const DEFAULT_PASSWORD: Option<String> = None;
    const DEFAULT_HOST: &'static str = "localhost";
    const DEFAULT_PORT: u16 = 3306;

    /// Creates a new privileged MySQL configuration
    /// # Example
    /// ```
    /// # use db_pool::PrivilegedMySQLConfig;
    /// #
    /// let config = PrivilegedMySQLConfig::new();
    /// ```
    /// # Defaults
    /// - Username: root
    /// - Password: {blank}
    /// - Host: localhost
    /// - Port: 3306
    #[must_use]
    pub fn new() -> Self {
        Self {
            username: Self::DEFAULT_USERNAME.to_owned(),
            password: Self::DEFAULT_PASSWORD,
            host: Self::DEFAULT_HOST.to_owned(),
            port: Self::DEFAULT_PORT,
        }
    }

    /// Creates a new privileged MySQL configuration from environment variables
    /// # Environment variables
    /// - `MYSQL_USERNAME`
    /// - `MYSQL_PASSWORD`
    /// - `MYSQL_HOST`
    /// - `MYSQL_PORT`
    /// # Defaults
    /// - Username: root
    /// - Password: {blank}
    /// - Host: localhost
    /// - Port: 3306
    pub fn from_env() -> Result<Self, Error> {
        use std::env;

        let username = env::var("MYSQL_USERNAME").unwrap_or(Self::DEFAULT_USERNAME.to_owned());
        let password = env::var("MYSQL_PASSWORD").ok();
        let host = env::var("MYSQL_HOST").unwrap_or(Self::DEFAULT_HOST.to_owned());
        let port = env::var("MYSQL_PORT")
            .map_or(Ok(Self::DEFAULT_PORT), |port| port.parse())
            .map_err(Error::InvalidPort)?;

        Ok(Self {
            username,
            password,
            host,
            port,
        })
    }

    /// Sets a new username
    /// # Example
    /// ```
    /// # use db_pool::PrivilegedMySQLConfig;
    /// #
    /// let config =
    ///     PrivilegedMySQLConfig::new().username("postgres".to_owned());
    /// ```
    #[must_use]
    pub fn username(self, value: String) -> Self {
        Self {
            username: value,
            ..self
        }
    }

    /// Sets a new password
    /// # Example
    /// ```
    /// # use db_pool::PrivilegedMySQLConfig;
    /// #
    /// let config = PrivilegedMySQLConfig::new().password(Some("root".to_owned()));
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
    /// let config = PrivilegedMySQLConfig::new().host("localhost".to_owned());
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
    /// let config = PrivilegedMySQLConfig::new().port(3306);
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
    InvalidPort(std::num::ParseIntError),
}

impl Default for PrivilegedMySQLConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "mysql")]
impl From<PrivilegedMySQLConfig> for r2d2_mysql::mysql::OptsBuilder {
    fn from(value: PrivilegedMySQLConfig) -> Self {
        Self::new()
            .user(Some(value.username.clone()))
            .pass(value.password.clone())
            .ip_or_hostname(Some(value.host.clone()))
            .tcp_port(value.port)
    }
}

#[cfg(feature = "mysql")]
impl From<PrivilegedMySQLConfig> for r2d2_mysql::mysql::Opts {
    fn from(value: PrivilegedMySQLConfig) -> Self {
        r2d2_mysql::mysql::OptsBuilder::from(value).into()
    }
}

#[cfg(feature = "sqlx-mysql")]
impl From<PrivilegedMySQLConfig> for sqlx::mysql::MySqlConnectOptions {
    fn from(value: PrivilegedMySQLConfig) -> Self {
        let PrivilegedMySQLConfig {
            username,
            password,
            host,
            port,
        } = value;

        let opts = Self::new()
            .username(username.as_str())
            .host(host.as_str())
            .port(port);

        if let Some(password) = password {
            opts.password(password.as_str())
        } else {
            opts
        }
    }
}
