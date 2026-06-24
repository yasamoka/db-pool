use std::{
    ffi::OsString,
    path::{PathBuf, absolute},
    str::FromStr,
};

use derive_more::Display;
use urlencoding::encode;

use crate::common::config::common::FromEnvError;

use super::common::{DatabaseEngine, PrivilegedConfig};

const USERNAME_ENV_VAR: &str = "POSTGRES_USERNAME";
const PASSWORD_ENV_VAR: &str = "POSTGRES_PASSWORD";
const HOST_ENV_VAR: &str = "POSTGRES_HOST";

/// Privileged Postgres configuration
pub type PrivilegedPostgresConfig = PrivilegedConfig<Postgres>;

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
    pub fn from_env() -> Result<Self, FromEnvError<Postgres>> {
        Self::from_env_inner()
    }
}

#[derive(Debug)]
pub struct Postgres;

impl DatabaseEngine for Postgres {
    const PREFIX: &str = "postgres";

    const DEFAULT_USERNAME: &str = "postgres";

    const USERNAME_ENV_VAR: &str = USERNAME_ENV_VAR;
    const PASSWORD_ENV_VAR: &str = PASSWORD_ENV_VAR;
    const HOST_ENV_VAR: &str = HOST_ENV_VAR;

    type HostConfig = PostgresHostConfig;
    type HostConfigInner = PostgresHostConfigInner;
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
        let PrivilegedPostgresConfig {
            username,
            password,
            host,
            _marker: _,
        } = value;

        let mut config = Self::new();

        config.user(username.as_str());

        match host {
            PostgresHostConfigInner::TcpIp { host, port } => {
                config.host(host.as_str()).port(port);
            }
            PostgresHostConfigInner::UnixSocket { socket } => {
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
            _marker,
        } = value;

        let opts = Self::new().username(username.as_str());

        let opts = match host {
            PostgresHostConfigInner::TcpIp { host, port } => opts.host(host.as_str()).port(port),
            PostgresHostConfigInner::UnixSocket { socket } => opts.host(&socket),
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
            _marker: _,
        } = value;

        let mut config = Self::new();

        config.user(username.as_str());

        match host {
            PostgresHostConfigInner::TcpIp { host, port } => {
                config.host(host).port(port);
            }
            PostgresHostConfigInner::UnixSocket { socket } => {
                config.host(socket);
            }
        }

        if let Some(password) = password {
            config.password(password.as_str());
        }

        config
    }
}
