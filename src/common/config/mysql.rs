use std::{
    ffi::OsString,
    path::{PathBuf, absolute},
    str::FromStr,
};

use derive_more::Display;
use urlencoding::encode;

use super::common::{DatabaseEngine, FromEnvError, PrivilegedConfig};

const USERNAME_ENV_VAR: &str = "MYSQL_USERNAME";
const PASSWORD_ENV_VAR: &str = "MYSQL_PASSWORD";
const HOST_ENV_VAR: &str = "MYSQL_HOST";

/// Privileged MySQL configuration
pub type PrivilegedMySQLConfig = PrivilegedConfig<MySQL>;

impl PrivilegedMySQLConfig {
    /// Creates a new privileged MySQL configuration from environment variables
    /// # Environment variables
    /// - `MYSQL_USERNAME`
    /// - `MYSQL_PASSWORD`
    /// - `MYSQL_HOST`
    /// # Defaults
    /// - Username: root
    /// - Password: {blank}
    /// - Host: localhost
    pub fn from_env() -> Result<Self, FromEnvError<MySQL>> {
        Self::from_env_inner()
    }
}

#[derive(Clone, Debug)]
pub struct MySQL;

impl DatabaseEngine for MySQL {
    const PREFIX: &str = "mysql";

    const DEFAULT_USERNAME: &str = "root";

    const USERNAME_ENV_VAR: &str = USERNAME_ENV_VAR;
    const PASSWORD_ENV_VAR: &str = PASSWORD_ENV_VAR;
    const HOST_ENV_VAR: &str = HOST_ENV_VAR;

    type HostConfig = MySQLHostConfig;
    type HostConfigInner = MySQLHostConfigInner;
}

pub enum MySQLHostConfig {
    Localhost,
    TcpIp { host: String, port: u16 },
    CustomUnixSocket(PathBuf),
}

#[derive(Clone, Debug, Default, Display)]
pub enum MySQLHostConfigInner {
    #[default]
    #[display("localhost")]
    Localhost,
    #[display("{host}:{port}")]
    TcpIp { host: String, port: u16 },
    #[display("{}", encode(socket))]
    CustomUnixSocket { socket: String },
}

impl MySQLHostConfigInner {
    pub(crate) fn host_name(&self) -> &str {
        match self {
            Self::Localhost | Self::CustomUnixSocket { .. } => "localhost",
            Self::TcpIp { host, port: _ } => host.as_str(),
        }
    }
}

impl FromStr for MySQLHostConfigInner {
    type Err = MySQLHostConfigFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use MySQLHostConfigFromStrError as E;

        if s == "localhost" {
            Ok(Self::Localhost)
        } else if s.starts_with('/') || s.starts_with("./") {
            Ok(Self::CustomUnixSocket {
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
pub enum MySQLHostConfigFromStrError {
    HostIsNotUnicode(OsString),
    InvalidHost(String),
    InvalidPort(std::num::ParseIntError),
}

impl TryFrom<MySQLHostConfig> for MySQLHostConfigInner {
    type Error = TryFromMySQLHostConfigError;

    fn try_from(value: MySQLHostConfig) -> Result<Self, Self::Error> {
        use TryFromMySQLHostConfigError as E;

        Ok(match value {
            MySQLHostConfig::Localhost => Self::Localhost,
            MySQLHostConfig::TcpIp { host, port } => Self::TcpIp { host, port },
            MySQLHostConfig::CustomUnixSocket(socket) => Self::CustomUnixSocket {
                socket: absolute(socket)
                    .map_err(E::Path)?
                    .to_str()
                    .ok_or(E::UnixSocketAddressIsNotUTF8)?
                    .to_owned(),
            },
        })
    }
}

pub enum TryFromMySQLHostConfigError {
    Path(std::io::Error),
    UnixSocketAddressIsNotUTF8,
}

#[cfg(feature = "mysql")]
impl From<PrivilegedMySQLConfig> for r2d2_mysql::mysql::OptsBuilder {
    fn from(value: PrivilegedMySQLConfig) -> Self {
        let PrivilegedMySQLConfig {
            username,
            password,
            host,
            _marker: _,
        } = value;

        let builder = Self::new()
            .user(Some(username.clone()))
            .pass(password.clone());

        match host {
            MySQLHostConfigInner::Localhost => builder.ip_or_hostname(Some("localhost")),
            MySQLHostConfigInner::TcpIp { host, port } => {
                builder.ip_or_hostname(Some(host)).tcp_port(port)
            }
            MySQLHostConfigInner::CustomUnixSocket { socket } => builder.socket(Some(socket)),
        }
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
            _marker: _,
        } = value;

        let mut opts = Self::new().username(username.as_str());

        if let Some(password) = password {
            opts = opts.password(password.as_str());
        }

        match host {
            MySQLHostConfigInner::Localhost => opts.host("localhost"),
            MySQLHostConfigInner::TcpIp { host, port } => opts.host(host.as_str()).port(port),
            MySQLHostConfigInner::CustomUnixSocket { socket } => opts.socket(socket),
        }
    }
}
