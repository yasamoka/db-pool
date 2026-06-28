use std::{
    ffi::OsString,
    path::{PathBuf, absolute},
    str::FromStr,
};

use derive_more::Display;

pub enum PostgresHostConfig {
    TcpIp { host: String, port: u16 },
    UnixSocket(PathBuf),
}

#[derive(Clone, Debug, Display)]
pub enum PostgresHostConfigInner {
    #[display("{host}:{port}")]
    TcpIp { host: String, port: u16 },
    #[display("{}", urlencoding::encode(socket))]
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
