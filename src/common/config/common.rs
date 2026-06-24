use std::{
    borrow::Cow,
    env::VarError,
    ffi::OsString,
    fmt::{Debug, Display},
    marker::PhantomData,
    str::FromStr,
};

use bon::Builder;

#[derive(Clone, Builder)]
pub struct PrivilegedConfig<DbEngine: DatabaseEngine> {
    #[builder(default = DbEngine::DEFAULT_USERNAME.to_owned())]
    pub(crate) username: String,
    pub(crate) password: Option<String>,
    #[builder(
        default = DbEngine::HostConfigInner::default(),
        with = |host: DbEngine::HostConfig| -> Result<
            _,
            <DbEngine::HostConfigInner as TryFrom<DbEngine::HostConfig>>::Error,
        > { DbEngine::HostConfigInner::try_from(host) }
    )]
    pub(crate) host: DbEngine::HostConfigInner,
    #[builder(skip)]
    pub(crate) _marker: PhantomData<DbEngine>,
}

impl<DbEngine: DatabaseEngine> PrivilegedConfig<DbEngine> {
    pub(super) fn from_env_inner() -> Result<Self, FromEnvError<DbEngine>> {
        use FromEnvError as E;
        use std::env;

        let username =
            env::var(DbEngine::USERNAME_ENV_VAR).unwrap_or(DbEngine::DEFAULT_USERNAME.to_owned());
        let password = env::var(DbEngine::PASSWORD_ENV_VAR).ok();

        let host = match env::var(DbEngine::HOST_ENV_VAR) {
            Ok(host) => host.parse().map_err(E::HostConfig),
            Err(VarError::NotPresent) => Ok(DbEngine::HostConfigInner::default()),
            Err(VarError::NotUnicode(s)) => Err(E::HostIsNotUnicode(s)),
        }?;

        Ok(Self {
            username,
            password,
            host,
            _marker: PhantomData,
        })
    }

    pub(crate) fn default_connection_url(&self) -> String {
        let Self {
            username, password, ..
        } = self;

        format!(
            "{}://{}@{}",
            DbEngine::PREFIX,
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
            DbEngine::PREFIX,
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
            DbEngine::PREFIX,
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
pub enum FromEnvError<DbEngine: DatabaseEngine> {
    HostIsNotUnicode(OsString),
    HostConfig(<DbEngine::HostConfigInner as FromStr>::Err),
}

impl<DbEngine: DatabaseEngine> Default for PrivilegedConfig<DbEngine> {
    fn default() -> Self {
        Self::builder().build()
    }
}

pub trait DatabaseEngine: Debug {
    const PREFIX: &str;

    const DEFAULT_USERNAME: &str;

    const USERNAME_ENV_VAR: &str;
    const PASSWORD_ENV_VAR: &str;
    const HOST_ENV_VAR: &str;

    type HostConfig;
    type HostConfigInner: Debug
        + FromStr<Err: Debug>
        + Default
        + Display
        + TryFrom<Self::HostConfig>;
}
