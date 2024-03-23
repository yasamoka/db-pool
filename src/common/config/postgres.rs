/// Privileged ``Postgres`` configuration
pub struct PrivilegedPostgresConfig {
    pub(crate) username: String,
    pub(crate) password: Option<String>,
    pub(crate) host: String,
    pub(crate) port: u16,
}

impl PrivilegedPostgresConfig {
    const DEFAULT_USERNAME: &'static str = "postgres";
    const DEFAULT_PASSWORD: Option<String> = None;
    const DEFAULT_HOST: &'static str = "localhost";
    const DEFAULT_PORT: u16 = 5432;

    /// Creates a new privileged ``Postgres`` configuration with defaults
    /// # Example
    /// ```
    /// # use db_pool::PrivilegedPostgresConfig;
    /// #
    /// let config = PrivilegedPostgresConfig::new();
    /// ```
    /// # Defaults
    /// - Username: postgres
    /// - Password: {blank}
    /// - Host: localhost
    /// - Port: 5432
    #[must_use]
    pub fn new() -> Self {
        Self {
            username: Self::DEFAULT_USERNAME.to_owned(),
            password: Self::DEFAULT_PASSWORD,
            host: Self::DEFAULT_HOST.to_owned(),
            port: Self::DEFAULT_PORT,
        }
    }

    /// Creates a new privileged ``Postgres`` configuration from environment variables
    /// # Environment variables
    /// - `POSTGRES_USERNAME`
    /// - `POSTGRES_PASSWORD`
    /// - `POSTGRES_HOST`
    /// - `POSTGRES_PORT`
    /// # Defaults
    /// - Username: postgres
    /// - Password: {blank}
    /// - Host: localhost
    /// - Port: 5432
    pub fn from_env() -> Result<Self, Error> {
        use std::env;

        let username = env::var("POSTGRES_USERNAME").unwrap_or(Self::DEFAULT_USERNAME.to_owned());
        let password = env::var("POSTGRES_PASSWORD").ok();
        let host = env::var("POSTGRES_HOST").unwrap_or(Self::DEFAULT_HOST.to_owned());
        let port = env::var("POSTGRES_PORT")
            .map_or(Ok(Self::DEFAULT_PORT), |port| port.parse())
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
    /// # use db_pool::PrivilegedPostgresConfig;
    /// #
    /// let config =
    ///     PrivilegedPostgresConfig::new().password(Some("postgres".to_owned()));
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
    /// # use db_pool::PrivilegedPostgresConfig;
    /// #
    /// let config = PrivilegedPostgresConfig::new().host("localhost".to_owned());
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
    /// # use db_pool::PrivilegedPostgresConfig;
    /// #
    /// let config = PrivilegedPostgresConfig::new().port(5432);
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
            format!("postgres://{username}:{password}@{host}:{port}")
        } else {
            format!("postgres://{username}@{host}:{port}")
        }
    }

    pub(crate) fn privileged_database_connection_url(&self, db_name: &str) -> String {
        let Self {
            username,
            password,
            host,
            port,
        } = self;
        if let Some(password) = password {
            format!("postgres://{username}:{password}@{host}:{port}/{db_name}")
        } else {
            format!("postgres://{username}@{host}:{port}/{db_name}")
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
            format!("postgres://{username}:{password}@{host}:{port}/{db_name}")
        } else {
            format!("postgres://{username}@{host}:{port}/{db_name}")
        }
    }
}

#[derive(Debug)]
pub enum Error {
    InvalidPort(std::num::ParseIntError),
}

impl Default for PrivilegedPostgresConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "postgres")]
impl From<PrivilegedPostgresConfig> for r2d2_postgres::postgres::Config {
    fn from(value: PrivilegedPostgresConfig) -> Self {
        let PrivilegedPostgresConfig {
            username,
            password,
            host,
            port,
        } = value;

        let mut config = Self::new();

        config
            .user(username.as_str())
            .host(host.as_str())
            .port(port);

        if let Some(password) = password {
            config.password(password.as_str());
        }

        config
    }
}

#[cfg(feature = "sqlx-postgres")]
impl From<PrivilegedPostgresConfig> for sqlx::postgres::PgConnectOptions {
    fn from(value: PrivilegedPostgresConfig) -> Self {
        let PrivilegedPostgresConfig {
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

#[cfg(feature = "tokio-postgres")]
impl From<PrivilegedPostgresConfig> for tokio_postgres::Config {
    fn from(value: PrivilegedPostgresConfig) -> Self {
        let PrivilegedPostgresConfig {
            username,
            password,
            host,
            port,
        } = value;

        let mut config = Self::new();

        config
            .user(username.as_str())
            .host(host.as_str())
            .port(port);

        if let Some(password) = password {
            config.password(password.as_str());
        }

        config
    }
}
