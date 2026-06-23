use std::{borrow::Cow, env::VarError, ffi::OsString, path::PathBuf};

use bon::Builder;
use urlencoding::encode;

const DEFAULT_USERNAME: &str = "postgres";
const DEFAULT_HOST: &str = "localhost";
const DEFAULT_PORT: u16 = 5432;

/// Privileged Postgres configuration
#[derive(Builder)]
pub struct PrivilegedPostgresConfig {
    #[builder(default = DEFAULT_USERNAME.to_owned())]
    username: String,
    password: Option<String>,
    #[builder(
        default = PostgresHostInner::TcpIp { host: DEFAULT_HOST.to_owned(), port: DEFAULT_PORT },
        with = |host: PostgresHost| -> Result<_, TryFromPostgresHostError> { PostgresHostInner::try_from(host) })]
    host: PostgresHostInner,
}

impl PrivilegedPostgresConfig {
    /// Creates a new privileged Postgres configuration from environment variables
    /// # Environment variables
    /// - `POSTGRES_USERNAME`
    /// - `POSTGRES_PASSWORD`
    /// - `POSTGRES_HOST`
    /// # Defaults
    /// - Username: postgres
    /// - Password: {blank}
    /// - Host: localhost:5432
    pub fn from_env() -> Result<Self, Error> {
        use std::env;

        let username = env::var("POSTGRES_USERNAME").unwrap_or(DEFAULT_USERNAME.to_owned());
        let password = env::var("POSTGRES_PASSWORD").ok();

        let host = match env::var("POSTGRES_HOST") {
            Ok(host) => {
                if host.starts_with('/') || host.starts_with("./") {
                    Ok(PostgresHostInner::UnixSocket(host))
                } else if let Some((host, port)) = host.split_once(':') {
                    port.parse()
                        .map(|port| PostgresHostInner::TcpIp {
                            host: host.to_owned(),
                            port,
                        })
                        .map_err(Error::InvalidPort)
                } else {
                    Err(Error::InvalidHost(host))
                }
            }
            Err(VarError::NotPresent) => Ok(PostgresHostInner::TcpIp {
                host: DEFAULT_HOST.to_owned(),
                port: DEFAULT_PORT,
            }),
            Err(VarError::NotUnicode(s)) => Err(Error::HostIsNotUnicode(s)),
        }?;

        Ok(Self {
            username,
            password,
            host,
        })
    }

    pub(crate) fn default_connection_url(&self) -> String {
        let Self {
            username, password, ..
        } = self;

        format!(
            "postgres://{}@{}",
            Self::credentials(username, password.as_ref().map(String::as_str)),
            self.host()
        )
    }

    pub(crate) fn privileged_database_connection_url(&self, db_name: &str) -> String {
        let Self {
            username, password, ..
        } = self;

        format!(
            "postgres://{}@{}/{}",
            Self::credentials(username, password.as_ref().map(String::as_str)),
            self.host(),
            db_name
        )
    }

    pub(crate) fn restricted_database_connection_url(
        &self,
        username: &str,
        password: Option<&str>,
        db_name: &str,
    ) -> String {
        format!(
            "postgres://{}@{}/{}",
            Self::credentials(username, password),
            self.host(),
            db_name
        )
    }

    fn credentials<'a>(username: &'a str, password: Option<&'a str>) -> Cow<'a, str> {
        if let Some(password) = password {
            Cow::Owned(format!("{username}:{password}"))
        } else {
            Cow::Borrowed(username)
        }
    }

    fn host(&self) -> Cow<'_, str> {
        match &self.host {
            PostgresHostInner::TcpIp { host, port } => Cow::Owned(format!("{host}:{port}")),
            PostgresHostInner::UnixSocket(socket) => encode(socket),
        }
    }
}

#[derive(Debug)]
pub enum Error {
    HostIsNotUnicode(OsString),
    InvalidHost(String),
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
        } = value;

        let mut config = Self::new();

        config.user(username.as_str());

        match host {
            PostgresHostInner::TcpIp { host, port } => {
                config.host(host.as_str()).port(port);
            }
            PostgresHostInner::UnixSocket(socket) => {
                config.host(&socket);
            }
        }

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
        } = value;

        let opts = Self::new().username(username.as_str());

        let opts = match host {
            PostgresHostInner::TcpIp { host, port } => opts.host(host.as_str()).port(port),
            PostgresHostInner::UnixSocket(socket) => opts.host(&socket),
        };

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
        } = value;

        let mut config = Self::new();

        config.user(username.as_str());

        match host {
            PostgresHostInner::TcpIp { host, port } => {
                config.host(host).port(port);
            }
            PostgresHostInner::UnixSocket(socket) => {
                config.host(socket);
            }
        }

        if let Some(password) = password {
            config.password(password.as_str());
        }

        config
    }
}

pub enum PostgresHost {
    TcpIp { host: String, port: u16 },
    UnixSocket(PathBuf),
}

enum PostgresHostInner {
    TcpIp { host: String, port: u16 },
    UnixSocket(String),
}

impl TryFrom<PostgresHost> for PostgresHostInner {
    type Error = TryFromPostgresHostError;

    fn try_from(value: PostgresHost) -> Result<Self, Self::Error> {
        Ok(match value {
            PostgresHost::TcpIp { host, port } => PostgresHostInner::TcpIp { host, port },
            PostgresHost::UnixSocket(socket) => PostgresHostInner::UnixSocket(
                socket
                    .to_str()
                    .ok_or(TryFromPostgresHostError::UnixSocketAddressIsNotUTF8)?
                    .to_owned(),
            ),
        })
    }
}

pub enum TryFromPostgresHostError {
    UnixSocketAddressIsNotUTF8,
}
