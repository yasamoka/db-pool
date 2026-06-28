use std::time::Duration;

use super::super::{
    config::PrivilegedPostgresConfig, host::PostgresHostConfigInner, params::Parameters,
};

impl From<PrivilegedPostgresConfig> for tokio_postgres::Config {
    #[allow(clippy::too_many_lines)]
    fn from(value: PrivilegedPostgresConfig) -> Self {
        let PrivilegedPostgresConfig {
            credentials,
            host,
            parameters,
        } = value;

        let mut config = Self::new();

        config.user(credentials.username());

        match host {
            PostgresHostConfigInner::TcpIp { host, port } => {
                config.host(host).port(port);
            }
            PostgresHostConfigInner::UnixSocket { socket } => {
                config.host(socket);
            }
        }

        if let Some(password) = credentials.password() {
            config.password(password);
        }

        {
            let Parameters {
                passfile: _,
                require_auth: _,
                channel_binding,
                connect_timeout,
                client_encoding: _,
                options,
                application_name,
                fallback_application_name: _,
                keepalives,
                keepalives_idle,
                keepalives_interval,
                keepalives_count,
                tcp_user_timeout,
                replication: _,
                gss_enc_mode: _,
                ssl_mode,
                require_ssl: _,
                ssl_negotiation,
                ssl_compression: _,
                ssl_cert: _,
                ssl_key: _,
                ssl_key_log_file: _,
                ssl_password: _,
                ssl_cert_mode: _,
                ssl_root_cert: _,
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
                target_session_attrs,
                load_balance_hosts,
                oauth_issuer: _,
                oauth_client_id: _,
                oauth_client_secret: _,
                oauth_scope: _,
            } = parameters;

            if let Some(channel_binding) = channel_binding {
                config.channel_binding(channel_binding.into());
            }

            if let Some(connect_timeout) = connect_timeout {
                config.connect_timeout(Duration::from_secs(connect_timeout));
            }

            if let Some(options) = options {
                config.options(options.to_string());
            }

            if let Some(application_name) = application_name {
                config.application_name(application_name);
            }

            if let Some(keepalives) = keepalives {
                config.keepalives(keepalives);
            }

            if let Some(keepalives_idle) = keepalives_idle {
                config.keepalives_idle(Duration::from_secs(keepalives_idle));
            }

            if let Some(keepalives_interval) = keepalives_interval {
                config.keepalives_interval(Duration::from_secs(keepalives_interval));
            }

            if let Some(keepalives_count) = keepalives_count {
                config.keepalives_retries(keepalives_count);
            }

            if let Some(tcp_user_timeout) = tcp_user_timeout {
                config.tcp_user_timeout(Duration::from_secs(tcp_user_timeout));
            }

            if let Some(ssl_mode) = ssl_mode
                && let Some(ssl_mode) = ssl_mode.into()
            {
                config.ssl_mode(ssl_mode);
            }

            if let Some(ssl_negotiation) = ssl_negotiation {
                config.ssl_negotiation(ssl_negotiation.into());
            }

            if let Some(target_session_attrs) = target_session_attrs
                && let Some(target_session_attrs) = target_session_attrs.into()
            {
                config.target_session_attrs(target_session_attrs);
            }

            if let Some(load_balance_hosts) = load_balance_hosts {
                config.load_balance_hosts(load_balance_hosts.into());
            }
        }

        config
    }
}
