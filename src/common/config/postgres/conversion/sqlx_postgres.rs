use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use super::{
    super::{
        config::PrivilegedPostgresConfig,
        host::PostgresHostConfigInner,
        param::{SslKey, SslMode, SslRootCert},
        params::{ParameterKey, Parameters},
    },
    common::TryFromPrivilegedPostgresConfigError,
};

impl TryFrom<PrivilegedPostgresConfig> for sqlx::postgres::PgConnectOptions {
    type Error = TryFromPrivilegedPostgresConfigError;

    #[allow(clippy::too_many_lines)]
    fn try_from(value: PrivilegedPostgresConfig) -> Result<Self, Self::Error> {
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

        let opts = if let Some(SslRootCert::Specific(ssl_root_cert)) = ssl_root_cert {
            opts.ssl_root_cert(ssl_root_cert)
        } else {
            opts
        };

        let unsupported_params = {
            use ParameterKey as P;
            [
                (passfile.is_some(), P::Passfile),
                (require_auth.is_some(), P::RequireAuth),
                (channel_binding.is_some(), P::ChannelBinding),
                (connect_timeout.is_some(), P::ConnectTimeout),
                (client_encoding.is_some(), P::ClientEncoding),
                (
                    fallback_application_name.is_some(),
                    P::FallbackApplicationName,
                ),
                (keepalives.is_some(), P::Keepalives),
                (keepalives_idle.is_some(), P::KeepalivesIdle),
                (keepalives_interval.is_some(), P::KeepalivesInterval),
                (keepalives_count.is_some(), P::KeepalivesCount),
                (tcp_user_timeout.is_some(), P::TcpUserTimeout),
                (replication.is_some(), P::Replication),
                (gss_enc_mode.is_some(), P::Gssencmode),
                (require_ssl.is_some(), P::Requiressl),
                (ssl_negotiation.is_some(), P::Sslnegotiation),
                (ssl_compression.is_some(), P::Sslcompression),
                (ssl_key_log_file.is_some(), P::Sslkeylogfile),
                (ssl_password.is_some(), P::Sslpassword),
                (ssl_cert_mode.is_some(), P::Sslcertmode),
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
                (target_session_attrs.is_some(), P::TargetSessionAttrs),
                (load_balance_hosts.is_some(), P::LoadBalanceHosts),
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
            Ok(opts)
        } else {
            Err(TryFromPrivilegedPostgresConfigError::UnsupportedParams(
                unsupported_params,
            ))
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
