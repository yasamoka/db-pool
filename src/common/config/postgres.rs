use url::Url;
use urlencoding::encode;

/// Privileged Postgres configuration
pub struct PrivilegedPostgresConfig {
    pub(crate) database_url: Option<String>,
    pub(crate) username: String,
    pub(crate) password: Option<String>,
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) options: Vec<(String, String)>,
}

impl PrivilegedPostgresConfig {
    const DEFAULT_USERNAME: &'static str = "postgres";
    const DEFAULT_PASSWORD: Option<String> = None;
    const DEFAULT_HOST: &'static str = "localhost";
    const DEFAULT_PORT: u16 = 5432;
    const DEFAULT_OPTIONS: Vec<(String, String)> = Vec::new();

    /// Creates a new privileged Postgres configuration with defaults
    /// # Example
    /// ```
    /// # use db_pool::PrivilegedPostgresConfig;
    /// #
    /// let config = PrivilegedPostgresConfig::new();
    /// ```
    /// # Defaults
    /// - Username: postgres
    /// - Password: {blank}
    /// - Host: localhost
    /// - Port: 5432
    #[must_use]
    pub fn new() -> Self {
        Self {
            database_url: None,
            username: Self::DEFAULT_USERNAME.to_owned(),
            password: Self::DEFAULT_PASSWORD,
            host: Self::DEFAULT_HOST.to_owned(),
            port: Self::DEFAULT_PORT,
            options: Self::DEFAULT_OPTIONS.clone(),
        }
    }

    /// Creates a new privileged Postgres configuration from environment variables
    /// # Environment variables
    /// - `POSTGRES_DATABASE_URL` (if set, overrides individual connection parameters)
    /// - `POSTGRES_USERNAME`
    /// - `POSTGRES_PASSWORD`
    /// - `POSTGRES_HOST`
    /// - `POSTGRES_PORT`
    /// - `POSTGRES_OPTIONS`
    /// # Defaults
    /// - Username: postgres
    /// - Password: {blank}
    /// - Host: localhost
    /// - Port: 5432
    pub fn from_env() -> Result<Self, Error> {
        use std::env;

        let database_url = env::var("POSTGRES_DATABASE_URL").ok();
        let username = env::var("POSTGRES_USERNAME").unwrap_or(Self::DEFAULT_USERNAME.to_owned());
        let password = env::var("POSTGRES_PASSWORD").ok();
        let host = env::var("POSTGRES_HOST").unwrap_or(Self::DEFAULT_HOST.to_owned());
        let port = env::var("POSTGRES_PORT")
            .map_or(Ok(Self::DEFAULT_PORT), |port| port.parse())
            .map_err(Error::InvalidPort)?;
        let options = Self::options_from_string(env::var("POSTGRES_OPTIONS").ok());

        Ok(Self {
            database_url,
            username,
            password,
            host,
            port,
            options
        })
    }

    /// Sets a new username
    /// # Example
    /// ```
    /// # use db_pool::PrivilegedPostgresConfig;
    /// #
    /// let config =
    ///     PrivilegedPostgresConfig::new().username("postgres".to_owned());
    /// ```
    #[must_use]
    pub fn username(self, value: String) -> Self {
        Self {
            username: value,
            ..self
        }
    }

    /// Sets a new password
    /// # Example
    /// ```
    /// # use db_pool::PrivilegedPostgresConfig;
    /// #
    /// let config =
    ///     PrivilegedPostgresConfig::new().password(Some("postgres".to_owned()));
    /// ```
    #[must_use]
    pub fn password(self, value: Option<String>) -> Self {
        Self {
            password: value,
            ..self
        }
    }

    /// Sets a new host
    /// # Example
    /// ```
    /// # use db_pool::PrivilegedPostgresConfig;
    /// #
    /// let config = PrivilegedPostgresConfig::new().host("localhost".to_owned());
    /// ```
    #[must_use]
    pub fn host(self, value: String) -> Self {
        Self {
            host: value,
            ..self
        }
    }

    /// Sets a new port
    /// # Example
    /// ```
    /// # use db_pool::PrivilegedPostgresConfig;
    /// #
    /// let config = PrivilegedPostgresConfig::new().port(5432);
    /// ```
    #[must_use]
    pub fn port(self, value: u16) -> Self {
        Self {
            port: value,
            ..self
        }
    }

    /// Sets options for connection string
    /// # Example
    /// ```
    /// # use db_pool::PrivilegedPostgresConfig;
    /// #
    /// let config = PrivilegedPostgresConfig::new().options(Vec::from([("option1".to_string(), "value1".to_string()),
    ///   ("option2".to_string(), "value2".to_string())]));
    /// ```
    #[must_use]
    pub fn options(self, value: Vec<(String, String)>) -> Self {
        Self {
            options: value,
            ..self
        }
    }

    /// Sets a complete database URL (overrides individual connection parameters)
    /// Useful for Unix socket connections and complex connection strings
    #[must_use]
    pub fn database_url(self, value: String) -> Self {
        Self {
            database_url: Some(value),
            ..self
        }
    }

    pub(crate) fn default_connection_url(&self) -> String {
        if let Some(database_url) = &self.database_url {
            return database_url.clone();
        }

        let Self {
            username,
            password,
            host,
            port,
            options,
            ..
        } = self;
        let opt_str = Self::options_to_segment(options);
        if let Some(password) = password {
            format!("postgres://{username}:{password}@{host}:{port}{opt_str}")
        } else {
            format!("postgres://{username}@{host}:{port}{opt_str}")
        }
    }

    pub(crate) fn privileged_database_connection_url(&self, db_name: &str) -> String {
        if let Some(database_url) = &self.database_url {
            return Self::replace_database_name_in_url(database_url, db_name);
        }

        let Self {
            username,
            password,
            host,
            port,
            options,
            ..
        } = self;

        let opt_str = Self::options_to_segment(options);
        if let Some(password) = password {
            format!("postgres://{username}:{password}@{host}:{port}/{db_name}{opt_str}")
        } else {
            format!("postgres://{username}@{host}:{port}/{db_name}{opt_str}")
        }
    }

    pub(crate) fn restricted_database_connection_url(
        &self,
        username: &str,
        password: Option<&str>,
        db_name: &str,
    ) -> String {
        if let Some(database_url) = &self.database_url {
            let url_with_credentials = Self::replace_credentials_in_url(database_url, username, password);
            return Self::replace_database_name_in_url(&url_with_credentials, db_name);
        }

        let Self { host, port, options, .. } = self;
        let opt_str = Self::options_to_segment(options);
        if let Some(password) = password {
            format!("postgres://{username}:{password}@{host}:{port}/{db_name}{opt_str}")
        } else {
            format!("postgres://{username}@{host}:{port}/{db_name}{opt_str}")
        }
    }

    pub(crate) fn options_to_string(options: &Vec<(String, String)>) -> String {
        let query: String = options
          .iter()
          .map(|(k, v)| format!("{}={}", k, v))
          .collect::<Vec<_>>()
          .join("&");
        if !query.is_empty() {
            format!("?{}", query)
        } else {
            String::new()
        }
    }

    pub(crate) fn options_from_string(os: Option<String>) -> Vec<(String, String)> {
        let s = match os {
            Some(s) => s,
            None => return Vec::new(),
        };
        let s = s.strip_prefix('?').unwrap_or(&s);
        if s.is_empty() {
            return Vec::new();
        }
        s.split('&')
          .filter_map(|pair| {
              let mut parts = pair.splitn(2, '=');
              let key = parts.next()?.to_string();
              let value = parts.next()?.to_string();
              if key.is_empty() {
                  None
              } else {
                  Some((key, value))
              }
          })
          .collect()
    }

    fn options_to_segment(options: &Vec<(String, String)>) -> String {
        if options.is_empty() {
            return String::new();
        }

        let options_merge = options
          .iter()
          .map(|(k, v)| format!("{}={}", k, v))
          .collect::<Vec<_>>();

        // Create the connection uri portion
        let options_segments = options_merge
          .iter()
          // The equal signs need to be encoded, since the url set_query doesn't do them,
          // and postgres requires them to be %3D
          // https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNSTRING
          .map(|o| format!("-c%20{}", encode(o)))
          .collect::<Vec<String>>()
          .join("%20");

        format!("?options={options_segments}")
    }

    /// Replace the database name in a PostgreSQL URL while preserving all other components
    fn replace_database_name_in_url(database_url: &str, new_db_name: &str) -> String {
        let mut url = Url::parse(database_url)
            .expect("Invalid database URL provided in POSTGRES_DATABASE_URL");
        url.set_path(&format!("/{}", new_db_name));
        url.to_string()
    }

    /// Replace credentials in a PostgreSQL URL while preserving all other components
    fn replace_credentials_in_url(database_url: &str, username: &str, password: Option<&str>) -> String {
        let mut url = Url::parse(database_url)
            .expect("Invalid database URL provided in POSTGRES_DATABASE_URL");
        url.set_username(username)
            .expect("Failed to set username in database URL");
        if let Some(password) = password {
            url.set_password(Some(password))
                .expect("Failed to set password in database URL");
        } else {
            url.set_password(None)
                .expect("Failed to clear password in database URL");
        }
        url.to_string()
    }
}

#[derive(Debug)]
pub enum Error {
    InvalidPort(std::num::ParseIntError),
}

impl Default for PrivilegedPostgresConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "postgres")]
impl From<PrivilegedPostgresConfig> for r2d2_postgres::postgres::Config {
    fn from(value: PrivilegedPostgresConfig) -> Self {
        let PrivilegedPostgresConfig {
            username,
            password,
            host,
            port,
            ..
        } = value;

        let mut config = Self::new();

        config
            .user(username.as_str())
            .host(host.as_str())
            .port(port);

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
            port,
            options,
            ..
        } = value;

        let opts = Self::new()
            .username(username.as_str())
            .host(host.as_str())
            .port(port)
            .options(options);

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
            port,
            ..
        } = value;

        let mut config = Self::new();

        config
            .user(username.as_str())
            .host(host.as_str())
            .port(port);

        if let Some(password) = password {
            config.password(password.as_str());
        }

        config
    }
}

#[test]
fn test_default_connection_url_with_options() {
    let config = PrivilegedPostgresConfig::new().options(vec![
        ("geqo".to_string(), "off".to_string()),
        ("statement_timeout".to_string(), "5min".to_string()),
    ]);
    let url = config.default_connection_url();
    assert_eq!(
        url,
        "postgres://postgres@localhost:5432?options=-c%20geqo%3Doff%20-c%20statement_timeout%3D5min"
    );
}

#[test]
fn test_database_url_override() {
    let config = PrivilegedPostgresConfig::new()
        .database_url("postgresql://user:password@%2Fvar%2Frun%2Fpostgresql/mydb".to_string());

    let url = config.default_connection_url();
    assert_eq!(url, "postgresql://user:password@%2Fvar%2Frun%2Fpostgresql/mydb");
}

#[test]
fn test_privileged_database_connection_url_with_database_url() {
    let config = PrivilegedPostgresConfig::new()
        .database_url("postgresql://user:password@%2Fvar%2Frun%2Fpostgresql/mydb".to_string());

    let url = config.privileged_database_connection_url("test_db");
    assert_eq!(url, "postgresql://user:password@%2Fvar%2Frun%2Fpostgresql/test_db");
}

#[test]
fn test_restricted_database_connection_url_with_database_url() {
    let config = PrivilegedPostgresConfig::new()
        .database_url("postgresql://admin:admin_pass@%2Fvar%2Frun%2Fpostgresql/original_db".to_string());

    let url = config.restricted_database_connection_url("restricted_user", Some("restricted_pass"), "restricted_db");
    assert_eq!(url, "postgresql://restricted_user:restricted_pass@%2Fvar%2Frun%2Fpostgresql/restricted_db");
}

