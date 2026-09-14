use std::{
    env::{VarError, var},
    ffi::OsString,
    str::FromStr,
};

use bon::bon;

use super::{
    super::credentials::Credentials,
    host::{PostgresHostConfig, PostgresHostConfigFromStrError, PostgresHostConfigInner},
    params::Parameters,
};

const PREFIX: &str = "postgres";
const DEFAULT_USERNAME: &str = "postgres";
const USERNAME_ENV_VAR: &str = "POSTGRES_USERNAME";
const PASSWORD_ENV_VAR: &str = "POSTGRES_PASSWORD";
const HOST_ENV_VAR: &str = "POSTGRES_HOST";
const PARAMS_ENV_VAR: &str = "POSTGRES_PARAMS";

/// Privileged Postgres configuration
pub struct PrivilegedPostgresConfig {
    pub(crate) credentials: Credentials<'static>,
    pub(crate) host: PostgresHostConfigInner,
    pub(super) parameters: Parameters,
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
        > { PostgresHostConfigInner::try_from(host) },
    )]
        host: PostgresHostConfigInner,
        #[builder(default = Parameters::builder().build())] parameters: Parameters,
    ) -> Self {
        Self {
            credentials: Credentials::new(username, password),
            host,
            parameters,
        }
    }

    /// Creates a new privileged Postgres configuration from environment variables
    /// # Environment variables
    /// - `POSTGRES_USERNAME`
    /// - `POSTGRES_PASSWORD`
    /// - `POSTGRES_HOST`
    /// - `POSTGRES_PARAMS`
    /// # Defaults
    /// - Username: postgres
    /// - Password: {blank}
    /// - Host: localhost:5432
    /// - Params: {blank}
    pub fn from_env() -> Result<Self, FromEnvError> {
        use FromEnvError as E;

        let username = var(USERNAME_ENV_VAR).unwrap_or(DEFAULT_USERNAME.to_owned());
        let password = var(PASSWORD_ENV_VAR).ok();

        let host = match var(HOST_ENV_VAR) {
            Ok(host) => host.parse().map_err(E::HostConfig),
            Err(VarError::NotPresent) => Ok(PostgresHostConfigInner::default()),
            Err(VarError::NotUnicode(s)) => Err(E::VarIsNotUnicode(s)),
        }?;

        let parameters = match var(PARAMS_ENV_VAR) {
            Ok(parameters) => parameters.parse().map_err(E::Parameters),
            Err(VarError::NotPresent) => Ok(Parameters::builder().build()),
            Err(VarError::NotUnicode(s)) => Err(E::VarIsNotUnicode(s)),
        }?;

        Ok(Self {
            credentials: Credentials::new(username, password),
            host,
            parameters,
        })
    }

    pub(crate) fn default_connection_url(&self) -> String {
        let Self {
            credentials,
            host,
            parameters,
        } = self;

        format!("{PREFIX}://{credentials}@{host}{parameters}")
    }

    pub(crate) fn privileged_database_connection_url(&self, db_name: &str) -> String {
        let Self {
            credentials,
            host,
            parameters,
        } = self;

        format!("{PREFIX}://{credentials}@{host}/{db_name}{parameters}")
    }

    pub(crate) fn restricted_database_connection_url(
        &self,
        username: &str,
        password: Option<&str>,
        db_name: &str,
    ) -> String {
        format!(
            "{}://{}@{}/{}{}",
            PREFIX,
            Credentials::new(username, password),
            self.host,
            db_name,
            self.parameters,
        )
    }
}

#[derive(Debug)]
pub enum FromEnvError {
    VarIsNotUnicode(OsString),
    HostConfig(PostgresHostConfigFromStrError),
    Parameters(<Parameters as FromStr>::Err),
}

#[cfg(test)]
mod tests {
    use super::PrivilegedPostgresConfig;
    use crate::common::config::postgres::Parameters;

    fn config_with_parameters() -> PrivilegedPostgresConfig {
        PrivilegedPostgresConfig::builder()
            .username("user".to_owned())
            .password("pass".to_owned())
            .parameters(
                Parameters::builder()
                    .application_name("app".to_owned())
                    .build(),
            )
            .build()
    }

    #[test]
    fn connection_url_includes_parameters() {
        let config = config_with_parameters();
        assert!(
            config
                .default_connection_url()
                .contains("application_name=app")
        );
        assert!(
            config
                .privileged_database_connection_url("db")
                .contains("application_name=app")
        );
        assert!(
            config
                .restricted_database_connection_url(
                    "restricted_user",
                    Some("restricted_pass"),
                    "db"
                )
                .contains("application_name=app")
        );
    }
}
