use std::{
    env::{VarError, var},
    ffi::OsString,
    path::{PathBuf, absolute},
    str::FromStr,
};

use bon::bon;
use derive_more::Display;
use urlencoding::encode;

use super::credentials::Credentials;

const PREFIX: &str = "postgres";
const DEFAULT_USERNAME: &str = "postgres";
const USERNAME_ENV_VAR: &str = "POSTGRES_USERNAME";
const PASSWORD_ENV_VAR: &str = "POSTGRES_PASSWORD";
const HOST_ENV_VAR: &str = "POSTGRES_HOST";

/// Privileged Postgres configuration
pub struct PrivilegedPostgresConfig {
    pub(crate) credentials: Credentials<'static>,
    pub(crate) host: PostgresHostConfigInner,
}

#[bon]
impl PrivilegedPostgresConfig {
    /// Build privileged Postgres configuration
    #[builder]
    pub fn new(
        #[builder(default = DEFAULT_USERNAME.to_owned())] username: String,
        password: Option<String>,
        #[builder(
        default = PostgresHostConfigInner::default(),
        with = |host: PostgresHostConfig| -> Result<
            _,
            <PostgresHostConfigInner as TryFrom<PostgresHostConfig>>::Error,
        > { PostgresHostConfigInner::try_from(host) }
    )]
        host: PostgresHostConfigInner,
    ) -> Self {
        Self {
            credentials: Credentials::new(username, password),
            host,
        }
    }

    /// Creates a new privileged Postgres configuration from environment variables
    /// # Environment variables
    /// - `POSTGRES_USERNAME`
    /// - `POSTGRES_PASSWORD`
    /// - `POSTGRES_HOST`
    /// # Defaults
    /// - Username: postgres
    /// - Password: {blank}
    /// - Host: localhost:5432
    pub fn from_env() -> Result<Self, FromEnvError> {
        use FromEnvError as E;

        let username = var(USERNAME_ENV_VAR).unwrap_or(DEFAULT_USERNAME.to_owned());
        let password = var(PASSWORD_ENV_VAR).ok();

        let host = match var(HOST_ENV_VAR) {
            Ok(host) => host.parse().map_err(E::HostConfig),
            Err(VarError::NotPresent) => Ok(PostgresHostConfigInner::default()),
            Err(VarError::NotUnicode(s)) => Err(E::HostIsNotUnicode(s)),
        }?;

        Ok(Self {
            credentials: Credentials::new(username, password),
            host,
        })
    }

    pub(crate) fn default_connection_url(&self) -> String {
        let Self { credentials, host } = self;

        format!("{PREFIX}://{credentials}@{host}")
    }

    pub(crate) fn privileged_database_connection_url(&self, db_name: &str) -> String {
        let Self { credentials, host } = self;

        format!("{PREFIX}://{credentials}@{host}/{db_name}")
    }

    pub(crate) fn restricted_database_connection_url(
        &self,
        username: &str,
        password: Option<&str>,
        db_name: &str,
    ) -> String {
        format!(
            "{}://{}@{}/{}",
            PREFIX,
            Credentials::new(username, password),
            self.host,
            db_name
        )
    }
}

#[derive(Debug)]
pub enum FromEnvError {
    HostIsNotUnicode(OsString),
    HostConfig(PostgresHostConfigFromStrError),
}

pub enum PostgresHostConfig {
    TcpIp { host: String, port: u16 },
    UnixSocket(PathBuf),
}

#[derive(Clone, Debug, Display)]
pub enum PostgresHostConfigInner {
    #[display("{host}:{port}")]
    TcpIp { host: String, port: u16 },
    #[display("{}", encode(socket))]
    UnixSocket { socket: String },
}

impl Default for PostgresHostConfigInner {
    fn default() -> Self {
        Self::TcpIp {
            host: "localhost".to_owned(),
            port: 5432,
        }
    }
}

impl FromStr for PostgresHostConfigInner {
    type Err = PostgresHostConfigFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use PostgresHostConfigFromStrError as E;

        if s.starts_with('/') || s.starts_with("./") {
            Ok(Self::UnixSocket {
                socket: s.to_owned(),
            })
        } else if let Some((host, port)) = s.split_once(':') {
            port.parse()
                .map(|port| Self::TcpIp {
                    host: host.to_owned(),
                    port,
                })
                .map_err(E::InvalidPort)
        } else {
            Err(E::InvalidHost(s.to_owned()))
        }
    }
}

#[derive(Debug)]
pub enum PostgresHostConfigFromStrError {
    HostIsNotUnicode(OsString),
    InvalidHost(String),
    InvalidPort(std::num::ParseIntError),
}

impl TryFrom<PostgresHostConfig> for PostgresHostConfigInner {
    type Error = TryFromPostgresHostConfigError;

    fn try_from(value: PostgresHostConfig) -> Result<Self, Self::Error> {
        use TryFromPostgresHostConfigError as E;

        Ok(match value {
            PostgresHostConfig::TcpIp { host, port } => {
                PostgresHostConfigInner::TcpIp { host, port }
            }
            PostgresHostConfig::UnixSocket(socket) => PostgresHostConfigInner::UnixSocket {
                socket: absolute(socket)
                    .map_err(E::Path)?
                    .to_str()
                    .ok_or(E::UnixSocketAddressIsNotUTF8)?
                    .to_owned(),
            },
        })
    }
}

pub enum TryFromPostgresHostConfigError {
    Path(std::io::Error),
    UnixSocketAddressIsNotUTF8,
}

#[cfg(feature = "postgres")]
impl From<PrivilegedPostgresConfig> for r2d2_postgres::postgres::Config {
    fn from(value: PrivilegedPostgresConfig) -> Self {
        let PrivilegedPostgresConfig { credentials, host } = value;

        let mut config = Self::new();

        config.user(credentials.username());

        match host {
            PostgresHostConfigInner::TcpIp { host, port } => {
                config.host(host.as_str()).port(port);
            }
            PostgresHostConfigInner::UnixSocket { socket } => {
                config.host(&socket);
            }
        }

        if let Some(password) = credentials.password() {
            config.password(password);
        }

        config
    }
}

#[cfg(feature = "sqlx-postgres")]
impl From<PrivilegedPostgresConfig> for sqlx::postgres::PgConnectOptions {
    fn from(value: PrivilegedPostgresConfig) -> Self {
        let PrivilegedPostgresConfig { credentials, host } = value;

        let opts = Self::new().username(credentials.username());

        let opts = match host {
            PostgresHostConfigInner::TcpIp { host, port } => opts.host(host.as_str()).port(port),
            PostgresHostConfigInner::UnixSocket { socket } => opts.host(&socket),
        };

        if let Some(password) = credentials.password() {
            opts.password(password)
        } else {
            opts
        }
    }
}

#[cfg(feature = "tokio-postgres")]
impl From<PrivilegedPostgresConfig> for tokio_postgres::Config {
    fn from(value: PrivilegedPostgresConfig) -> Self {
        let PrivilegedPostgresConfig { credentials, host } = value;

        let mut config = Self::new();

        config.user(credentials.username());

        match host {
            PostgresHostConfigInner::TcpIp { host, port } => {
                config.host(host).port(port);
            }
            PostgresHostConfigInner::UnixSocket { socket } => {
                config.host(socket);
            }
        }

        if let Some(password) = credentials.password() {
            config.password(password);
        }

        config
    }
}
