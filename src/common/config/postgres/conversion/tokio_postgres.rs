use std::{collections::HashSet, time::Duration};

use super::{
    super::{
        config::PrivilegedPostgresConfig,
        host::PostgresHostConfigInner,
        params::{ParameterKey, Parameters},
    },
    common::TryFromPrivilegedPostgresConfigError,
};

impl TryFrom<PrivilegedPostgresConfig> for tokio_postgres::Config {
    type Error = TryFromPrivilegedPostgresConfigError;

    #[allow(clippy::too_many_lines)]
    fn try_from(value: PrivilegedPostgresConfig) -> Result<Self, Self::Error> {
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

        let Parameters {
            passfile,
            require_auth,
            channel_binding,
            connect_timeout,
            client_encoding,
            options,
            application_name,
            fallback_application_name,
            keepalives,
            keepalives_idle,
            keepalives_interval,
            keepalives_count,
            tcp_user_timeout,
            replication,
            gss_enc_mode,
            ssl_mode,
            require_ssl,
            ssl_negotiation,
            ssl_compression,
            ssl_cert,
            ssl_key,
            ssl_key_log_file,
            ssl_password,
            ssl_cert_mode,
            ssl_root_cert,
            ssl_crl,
            ssl_crl_dir,
            ssl_sni,
            require_peer,
            ssl_min_protocol_version,
            ssl_max_protocol_version,
            min_protocol_version,
            max_protocol_version,
            krb_srv_name,
            gss_lib,
            gss_delegation,
            scram_client_key,
            scram_server_key,
            service,
            target_session_attrs,
            load_balance_hosts,
            oauth_issuer,
            oauth_client_id,
            oauth_client_secret,
            oauth_scope,
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

        let unsupported_params = {
            use ParameterKey as P;
            [
                (passfile.is_some(), P::Passfile),
                (require_auth.is_some(), P::RequireAuth),
                (client_encoding.is_some(), P::ClientEncoding),
                (
                    fallback_application_name.is_some(),
                    P::FallbackApplicationName,
                ),
                (replication.is_some(), P::Replication),
                (gss_enc_mode.is_some(), P::Gssencmode),
                (require_ssl.is_some(), P::Requiressl),
                (ssl_compression.is_some(), P::Sslcompression),
                (ssl_cert.is_some(), P::Sslcert),
                (ssl_key.is_some(), P::Sslkey),
                (ssl_key_log_file.is_some(), P::Sslkeylogfile),
                (ssl_password.is_some(), P::Sslpassword),
                (ssl_cert_mode.is_some(), P::Sslcertmode),
                (ssl_root_cert.is_some(), P::Sslrootcert),
                (ssl_crl.is_some(), P::Sslcrl),
                (ssl_crl_dir.is_some(), P::Sslcrldir),
                (ssl_sni.is_some(), P::Sslsni),
                (require_peer.is_some(), P::Requirepeer),
                (ssl_min_protocol_version.is_some(), P::SslMinProtocolVersion),
                (ssl_max_protocol_version.is_some(), P::SslMaxProtocolVersion),
                (min_protocol_version.is_some(), P::MinProtocolVersion),
                (max_protocol_version.is_some(), P::MaxProtocolVersion),
                (krb_srv_name.is_some(), P::Krbsrvname),
                (gss_lib.is_some(), P::Gsslib),
                (gss_delegation.is_some(), P::Gssdelegation),
                (scram_client_key.is_some(), P::ScramClientKey),
                (scram_server_key.is_some(), P::ScramServerKey),
                (service.is_some(), P::Service),
                (oauth_issuer.is_some(), P::OauthIssuer),
                (oauth_client_id.is_some(), P::OauthClientId),
                (oauth_client_secret.is_some(), P::OauthClientSecret),
                (oauth_scope.is_some(), P::OauthScope),
            ]
            .into_iter()
            .filter_map(|(is_set, key)| if is_set { Some(key) } else { None })
            .collect::<HashSet<_>>()
        };

        if unsupported_params.is_empty() {
            Ok(config)
        } else {
            Err(TryFromPrivilegedPostgresConfigError::UnsupportedParams(
                unsupported_params,
            ))
        }
    }
}
