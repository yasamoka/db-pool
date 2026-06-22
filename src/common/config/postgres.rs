use bon::Builder;

const DEFAULT_USERNAME: &str = "postgres";
const DEFAULT_HOST: &str = "localhost";
const DEFAULT_PORT: u16 = 5432;

/// Privileged Postgres configuration
#[derive(Builder)]
pub struct PrivilegedPostgresConfig {
    #[builder(default = DEFAULT_USERNAME.to_owned())]
    pub(crate) username: String,
    pub(crate) password: Option<String>,
    #[builder(default = DEFAULT_HOST.to_owned())]
    pub(crate) host: String,
    #[builder(default = DEFAULT_PORT)]
    pub(crate) port: u16,
}

impl PrivilegedPostgresConfig {
    /// Creates a new privileged Postgres configuration from environment variables
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

        let username = env::var("POSTGRES_USERNAME").unwrap_or(DEFAULT_USERNAME.to_owned());
        let password = env::var("POSTGRES_PASSWORD").ok();
        let host = env::var("POSTGRES_HOST").unwrap_or(DEFAULT_HOST.to_owned());
        let port = env::var("POSTGRES_PORT")
            .map_or(Ok(DEFAULT_PORT), |port| port.parse())
            .map_err(Error::InvalidPort)?;

        Ok(Self {
            username,
            password,
            host,
            port,
        })
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
        Self::builder().build()
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
