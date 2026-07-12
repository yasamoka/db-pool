use super::super::{config::PrivilegedMySQLConfig, host::MySQLHostConfigInner};

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
