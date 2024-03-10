pub struct PrivilegedConfig {
    pub(crate) username: String,
    pub(crate) password: Option<String>,
    pub(crate) host: String,
    pub(crate) port: u16,
}

impl PrivilegedConfig {
    #[must_use]
    pub fn new(username: String) -> Self {
        Self {
            username,
            password: None,
            host: "localhost".to_owned(),
            port: 3306,
        }
    }

    #[must_use]
    pub fn password(self, value: Option<String>) -> Self {
        Self {
            password: value,
            ..self
        }
    }

    #[must_use]
    pub fn host(self, value: String) -> Self {
        Self {
            host: value,
            ..self
        }
    }

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
            format!("mysql://{username}:{password}@{host}:{port}")
        } else {
            format!("mysql://{username}@{host}:{port}")
        }
    }

    pub(crate) fn privileged_database_connection_url(&self, db_name: &str) -> String {
        let Self {
            username,
            password,
            host,
            port,
            ..
        } = self;
        if let Some(password) = password {
            format!("mysql://{username}:{password}@{host}:{port}/{db_name}")
        } else {
            format!("mysql://{username}@{host}:{port}/{db_name}")
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
            format!("mysql://{username}:{password}@{host}:{port}/{db_name}")
        } else {
            format!("mysql://{username}@{host}:{port}/{db_name}")
        }
    }
}
