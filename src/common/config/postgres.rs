/// Privileged ``Postgres`` configuration
pub struct PrivilegedPostgresConfig {
    pub(crate) username: String,
    pub(crate) password: Option<String>,
    pub(crate) host: String,
    pub(crate) port: u16,
}

impl PrivilegedPostgresConfig {
    /// Creates a new privileged ``Postgres`` configuration
    /// # Example
    /// ```
    /// # use db_pool::PrivilegedPostgresConfig;
    /// #
    /// let config = PrivilegedPostgresConfig::new("postgres".to_owned());
    /// ```
    /// # Defaults
    /// * Password: {empty}
    /// * Host: localhost
    /// * Port: 5432
    #[must_use]
    pub fn new(username: String) -> Self {
        Self {
            username,
            password: None,
            host: "localhost".to_owned(),
            port: 5432,
        }
    }

    /// Sets a new password
    /// # Example
    /// ```
    /// # use db_pool::PrivilegedPostgresConfig;
    /// #
    /// let config =
    ///     PrivilegedPostgresConfig::new("postgres".to_owned()).password(Some("postgres".to_owned()));
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
    /// let config = PrivilegedPostgresConfig::new("postgres".to_owned()).host("localhost".to_owned());
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
    /// let config = PrivilegedPostgresConfig::new("postgres".to_owned()).port(5432);
    /// ```
    #[must_use]
    pub fn port(self, value: u16) -> Self {
        Self {
            port: value,
            ..self
        }
    }

    pub(crate) fn default_connection_url(&self) -> String {
        let Self {
            username,
            password,
            host,
            port,
        } = self;
        if let Some(password) = password {
            format!("postgres://{username}:{password}@{host}:{port}")
        } else {
            format!("postgres://{username}@{host}:{port}")
        }
    }

    pub(crate) fn privileged_database_connection_url(&self, db_name: &str) -> String {
        let Self {
            username,
            password,
            host,
            port,
        } = self;
        if let Some(password) = password {
            format!("postgres://{username}:{password}@{host}:{port}/{db_name}")
        } else {
            format!("postgres://{username}@{host}:{port}/{db_name}")
        }
    }

    pub(crate) fn restricted_database_connection_url(
        &self,
        username: &str,
        password: Option<&str>,
        db_name: &str,
    ) -> String {
        let Self { host, port, .. } = self;
        if let Some(password) = password {
            format!("postgres://{username}:{password}@{host}:{port}/{db_name}")
        } else {
            format!("postgres://{username}@{host}:{port}/{db_name}")
        }
    }
}
