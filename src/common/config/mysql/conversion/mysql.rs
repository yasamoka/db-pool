use super::super::{config::PrivilegedMySQLConfig, host::MySQLHostConfigInner};

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

impl From<PrivilegedMySQLConfig> for r2d2_mysql::mysql::Opts {
    fn from(value: PrivilegedMySQLConfig) -> Self {
        r2d2_mysql::mysql::OptsBuilder::from(value).into()
    }
}
