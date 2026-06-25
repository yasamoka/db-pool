use std::{
    borrow::Cow,
    env::{VarError, var},
    ffi::OsString,
    path::{PathBuf, absolute},
    str::FromStr,
};

use bon::bon;
use derive_more::Display;
use urlencoding::encode;

const PREFIX: &str = "mysql";
const DEFAULT_USERNAME: &str = "root";
const USERNAME_ENV_VAR: &str = "MYSQL_USERNAME";
const PASSWORD_ENV_VAR: &str = "MYSQL_PASSWORD";
const HOST_ENV_VAR: &str = "MYSQL_HOST";

/// Privileged MySQL configuration
#[derive(Clone)]
pub struct PrivilegedMySQLConfig {
    pub(crate) username: String,
    pub(crate) password: Option<String>,
    pub(crate) host: MySQLHostConfigInner,
}

#[bon]
impl PrivilegedMySQLConfig {
    /// Build privileged MySQL configuration
    #[builder]
    pub fn new(
        #[builder(default = DEFAULT_USERNAME.to_owned())] username: String,
        password: Option<String>,
        #[builder(
        default = MySQLHostConfigInner::default(),
            with = |host: MySQLHostConfig| -> Result<
                _,
                <MySQLHostConfigInner as TryFrom<MySQLHostConfig>>::Error,
            > { MySQLHostConfigInner::try_from(host) }
        )]
        host: MySQLHostConfigInner,
    ) -> Self {
        Self {
            username,
            password,
            host,
        }
    }

    /// Creates a new privileged MySQL configuration from environment variables
    /// # Environment variables
    /// - `MYSQL_USERNAME`
    /// - `MYSQL_PASSWORD`
    /// - `MYSQL_HOST`
    /// # Defaults
    /// - Username: root
    /// - Password: {blank}
    /// - Host: localhost
    pub fn from_env() -> Result<Self, FromEnvError> {
        use FromEnvError as E;

        let username = var(USERNAME_ENV_VAR).unwrap_or(DEFAULT_USERNAME.to_owned());
        let password = var(PASSWORD_ENV_VAR).ok();

        let host = match var(HOST_ENV_VAR) {
            Ok(host) => host.parse().map_err(E::HostConfig),
            Err(VarError::NotPresent) => Ok(MySQLHostConfigInner::default()),
            Err(VarError::NotUnicode(s)) => Err(E::HostIsNotUnicode(s)),
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
            "{}://{}@{}",
            PREFIX,
            Self::credentials(username, password.as_ref().map(String::as_str)),
            self.host
        )
    }

    pub(crate) fn privileged_database_connection_url(&self, db_name: &str) -> String {
        let Self {
            username, password, ..
        } = self;

        format!(
            "{}://{}@{}/{}",
            PREFIX,
            Self::credentials(username, password.as_ref().map(String::as_str)),
            self.host,
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
            "{}://{}@{}/{}",
            PREFIX,
            Self::credentials(username, password),
            self.host,
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
}

#[derive(Debug)]
pub enum FromEnvError {
    HostIsNotUnicode(OsString),
    HostConfig(MySQLHostConfigFromStrError),
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
