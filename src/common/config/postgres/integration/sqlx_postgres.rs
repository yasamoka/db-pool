use std::{collections::HashMap, path::PathBuf};

use super::super::{
    config::PrivilegedPostgresConfig,
    host::PostgresHostConfigInner,
    param::{SslKey, SslMode, SslRootCert},
    params::Parameters,
};

impl From<PrivilegedPostgresConfig> for sqlx::postgres::PgConnectOptions {
    #[allow(clippy::too_many_lines)]
    fn from(value: PrivilegedPostgresConfig) -> Self {
        let PrivilegedPostgresConfig {
            credentials,
            host,
            parameters,
        } = value;

        let opts = Self::new().username(credentials.username());

        let opts = match host {
            PostgresHostConfigInner::TcpIp { host, port } => opts.host(host.as_str()).port(port),
            PostgresHostConfigInner::UnixSocket { socket } => opts.host(&socket),
        };

        let opts = if let Some(password) = credentials.password() {
            opts.password(password)
        } else {
            opts
        };

        {
            let Parameters {
                passfile: _,
                require_auth: _,
                channel_binding: _,
                connect_timeout: _,
                client_encoding: _,
                options,
                application_name,
                fallback_application_name: _,
                keepalives: _,
                keepalives_idle: _,
                keepalives_interval: _,
                keepalives_count: _,
                tcp_user_timeout: _,
                replication: _,
                gss_enc_mode: _,
                ssl_mode,
                require_ssl: _,
                ssl_negotiation: _,
                ssl_compression: _,
                ssl_cert,
                ssl_key,
                ssl_key_log_file: _,
                ssl_password: _,
                ssl_cert_mode: _,
                ssl_root_cert,
                ssl_crl: _,
                ssl_crl_dir: _,
                ssl_sni: _,
                require_peer: _,
                ssl_min_protocol_version: _,
                ssl_max_protocol_version: _,
                min_protocol_version: _,
                max_protocol_version: _,
                krb_srv_name: _,
                gss_lib: _,
                gss_delegation: _,
                scram_client_key: _,
                scram_server_key: _,
                service: _,
                target_session_attrs: _,
                load_balance_hosts: _,
                oauth_issuer: _,
                oauth_client_id: _,
                oauth_client_secret: _,
                oauth_scope: _,
            } = parameters;

            let opts = if let Some(options) = options {
                opts.options(HashMap::from(options))
            } else {
                opts
            };

            let opts = if let Some(application_name) = application_name {
                opts.application_name(application_name.as_str())
            } else {
                opts
            };

            let opts = if let Some(ssl_mode) = ssl_mode {
                opts.ssl_mode(ssl_mode.into())
            } else {
                opts
            };

            let opts = if let Some(ssl_cert) = ssl_cert {
                opts.ssl_client_cert(PathBuf::from(ssl_cert))
            } else {
                opts
            };

            let opts = if let Some(SslKey::Path(ssl_key)) = ssl_key {
                opts.ssl_client_key(ssl_key)
            } else {
                opts
            };

            if let Some(SslRootCert::Specific(ssl_root_cert)) = ssl_root_cert {
                opts.ssl_root_cert(ssl_root_cert)
            } else {
                opts
            }
        }
    }
}

impl From<SslMode> for sqlx::postgres::PgSslMode {
    fn from(value: SslMode) -> Self {
        use SslMode as T;
        use sqlx::postgres::PgSslMode as U;
        match value {
            T::Allow => U::Allow,
            T::Disable => U::Disable,
            T::Prefer => U::Prefer,
            T::Require => U::Require,
            T::VerifyCa => U::VerifyCa,
            T::VerifyFull => U::VerifyFull,
        }
    }
}
