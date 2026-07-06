use std::{
    ffi::OsString,
    path::{PathBuf, absolute},
    str::FromStr,
};

use derive_more::Display;

/// MySQL host configuration
pub enum MySQLHostConfig {
    /// localhost
    Localhost,
    /// TCP/IP
    TcpIp {
        /// Host
        host: String,
        /// Port
        port: u16,
    },
    /// Custom Unix socket
    CustomUnixSocket(PathBuf),
}

#[derive(Clone, Debug, Default, Display)]
pub enum MySQLHostConfigInner {
    #[default]
    #[display("localhost")]
    Localhost,
    #[display("{host}:{port}")]
    TcpIp { host: String, port: u16 },
    #[display("{}", urlencoding::encode(socket))]
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
                // TODO: should absolute path be the responsibility of this conversion?
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
