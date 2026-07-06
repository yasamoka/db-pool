use std::{
    borrow::Cow,
    env::{VarError, var},
    ffi::OsString,
};

use bon::bon;

use super::host::{MySQLHostConfig, MySQLHostConfigFromStrError, MySQLHostConfigInner};

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
