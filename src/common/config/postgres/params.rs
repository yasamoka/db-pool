use std::{
    fmt::{Display, Write},
    str::FromStr,
    string::FromUtf8Error,
};

use bon::Builder;
use derive_more::{Display, From, FromStr};

use super::param::{
    AllowStrictness, AuthMethod, ClientEncoding, GssLib, HttpsUrl, Key, LoadBalanceHosts,
    NonEmptyString, OAuthScope, Options, ProtocolVersion, Replication, RequireStrictness,
    SecretString, SslKey, SslMode, SslNegotiation, SslProtocolVersion, SslRootCert,
    TargetSessionAttrs, UTF8Path,
};

/// Postgres parameters
#[derive(Builder, Debug, Eq, PartialEq)]
pub struct Parameters {
    pub(super) passfile: Option<UTF8Path>,
    pub(super) require_auth: Option<AuthMethod>,
    pub(super) channel_binding: Option<RequireStrictness>,
    pub(super) connect_timeout: Option<u64>,
    pub(super) client_encoding: Option<ClientEncoding>,
    pub(super) options: Option<Options>,
    pub(super) application_name: Option<String>,
    pub(super) fallback_application_name: Option<String>,
    pub(super) keepalives: Option<bool>,
    pub(super) keepalives_idle: Option<u64>,
    pub(super) keepalives_interval: Option<u64>,
    pub(super) keepalives_count: Option<u32>,
    pub(super) tcp_user_timeout: Option<u64>,
    pub(super) replication: Option<Replication>,
    pub(super) gss_enc_mode: Option<RequireStrictness>,
    pub(super) ssl_mode: Option<SslMode>,
    pub(super) require_ssl: Option<bool>,
    pub(super) ssl_negotiation: Option<SslNegotiation>,
    pub(super) ssl_compression: Option<bool>,
    pub(super) ssl_cert: Option<UTF8Path>,
    pub(super) ssl_key: Option<SslKey>,
    pub(super) ssl_key_log_file: Option<UTF8Path>,
    pub(super) ssl_password: Option<NonEmptyString>,
    pub(super) ssl_cert_mode: Option<AllowStrictness>,
    pub(super) ssl_root_cert: Option<SslRootCert>,
    pub(super) ssl_crl: Option<UTF8Path>,
    pub(super) ssl_crl_dir: Option<UTF8Path>,
    pub(super) ssl_sni: Option<bool>,
    pub(super) require_peer: Option<String>,
    pub(super) ssl_min_protocol_version: Option<SslProtocolVersion>,
    pub(super) ssl_max_protocol_version: Option<SslProtocolVersion>,
    pub(super) min_protocol_version: Option<ProtocolVersion>,
    pub(super) max_protocol_version: Option<ProtocolVersion>,
    pub(super) krb_srv_name: Option<String>,
    pub(super) gss_lib: Option<GssLib>,
    pub(super) gss_delegation: Option<bool>,
    pub(super) scram_client_key: Option<Key>,
    pub(super) scram_server_key: Option<Key>,
    pub(super) service: Option<String>,
    pub(super) target_session_attrs: Option<TargetSessionAttrs>,
    pub(super) load_balance_hosts: Option<LoadBalanceHosts>,
    pub(super) oauth_issuer: Option<HttpsUrl>,
    pub(super) oauth_client_id: Option<String>,
    pub(super) oauth_client_secret: Option<SecretString>,
    pub(super) oauth_scope: Option<OAuthScope>,
}

#[derive(Debug, Display, Eq, FromStr, Hash, PartialEq)]
#[from_str(rename_all = "snake_case")]
#[display(rename_all = "snake_case")]
pub enum ParameterKey {
    Passfile,
    RequireAuth,
    ChannelBinding,
    ConnectTimeout,
    ClientEncoding,
    Options,
    ApplicationName,
    FallbackApplicationName,
    Keepalives,
    KeepalivesIdle,
    KeepalivesInterval,
    KeepalivesCount,
    TcpUserTimeout,
    Replication,
    Gssencmode,
    Sslmode,
    Requiressl,
    Sslnegotiation,
    Sslcompression,
    Sslcert,
    Sslkey,
    Sslkeylogfile,
    Sslpassword,
    Sslcertmode,
    Sslrootcert,
    Sslcrl,
    Sslcrldir,
    Sslsni,
    Requirepeer,
    SslMinProtocolVersion,
    SslMaxProtocolVersion,
    MinProtocolVersion,
    MaxProtocolVersion,
    Krbsrvname,
    Gsslib,
    Gssdelegation,
    ScramClientKey,
    ScramServerKey,
    Service,
    TargetSessionAttrs,
    LoadBalanceHosts,
    OauthIssuer,
    OauthClientId,
    OauthClientSecret,
    OauthScope,
}

impl Display for Parameters {
    #[allow(clippy::too_many_lines)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ParameterKey as K;

        #[allow(clippy::ref_option)]
        fn v<T: ToString>(value: &Option<T>) -> Option<String> {
            value.as_ref().map(ToString::to_string)
        }

        let Self {
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
        } = self;

        let data = [
            (K::Passfile, v(passfile)),
            (K::RequireAuth, v(require_auth)),
            (K::ChannelBinding, v(channel_binding)),
            (K::ConnectTimeout, v(connect_timeout)),
            (K::ClientEncoding, v(client_encoding)),
            (K::Options, v(options)),
            (K::ApplicationName, v(application_name)),
            (K::FallbackApplicationName, v(fallback_application_name)),
            (K::Keepalives, v(keepalives)),
            (K::KeepalivesIdle, v(keepalives_idle)),
            (K::KeepalivesInterval, v(keepalives_interval)),
            (K::KeepalivesCount, v(keepalives_count)),
            (K::TcpUserTimeout, v(tcp_user_timeout)),
            (K::Replication, v(replication)),
            (K::Gssencmode, v(gss_enc_mode)),
            (K::Sslmode, v(ssl_mode)),
            (K::Requiressl, v(require_ssl)),
            (K::Sslnegotiation, v(ssl_negotiation)),
            (K::Sslcompression, v(ssl_compression)),
            (K::Sslcert, v(ssl_cert)),
            (K::Sslkey, v(ssl_key)),
            (K::Sslkeylogfile, v(ssl_key_log_file)),
            (K::Sslpassword, v(ssl_password)),
            (K::Sslcertmode, v(ssl_cert_mode)),
            (K::Sslrootcert, v(ssl_root_cert)),
            (K::Sslcrl, v(ssl_crl)),
            (K::Sslcrldir, v(ssl_crl_dir)),
            (K::Sslsni, v(ssl_sni)),
            (K::Requirepeer, v(require_peer)),
            (K::SslMinProtocolVersion, v(ssl_min_protocol_version)),
            (K::SslMaxProtocolVersion, v(ssl_max_protocol_version)),
            (K::MinProtocolVersion, v(min_protocol_version)),
            (K::MaxProtocolVersion, v(max_protocol_version)),
            (K::Krbsrvname, v(krb_srv_name)),
            (K::Gsslib, v(gss_lib)),
            (K::Gssdelegation, v(gss_delegation)),
            (K::ScramClientKey, v(scram_client_key)),
            (K::ScramServerKey, v(scram_server_key)),
            (K::Service, v(service)),
            (K::TargetSessionAttrs, v(target_session_attrs)),
            (K::LoadBalanceHosts, v(load_balance_hosts)),
            (K::OauthIssuer, v(oauth_issuer)),
            (K::OauthClientId, v(oauth_client_id)),
            (K::OauthClientSecret, v(oauth_client_secret)),
            (K::OauthScope, v(oauth_scope)),
        ]
        .into_iter()
        .filter_map(|(key, value)| {
            value.map(|value| format!("{key}={}", urlencoding::encode(value.as_str())))
        })
        .collect::<Vec<_>>()
        .join("&");

        if !data.is_empty() {
            f.write_char('?')?;
            f.write_str(&data)?;
        }

        Ok(())
    }
}

impl FromStr for Parameters {
    type Err = ParametersFromStrError;

    #[allow(clippy::too_many_lines)]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use ParametersFromStrError as E;

        type P<'a, T> = &'a mut Option<T>;

        #[derive(From)]
        enum Param<'a> {
            AllowStrictness(P<'a, AllowStrictness>),
            AuthMethod(P<'a, AuthMethod>),
            Bool(P<'a, bool>),
            ClientEncoding(P<'a, ClientEncoding>),
            GssLib(P<'a, GssLib>),
            HttpsUrl(P<'a, HttpsUrl>),
            Key(P<'a, Key>),
            LoadBalanceHosts(P<'a, LoadBalanceHosts>),
            NonEmptyString(P<'a, NonEmptyString>),
            OAuthScope(P<'a, OAuthScope>),
            Options(P<'a, Options>),
            ProtocolVersion(P<'a, ProtocolVersion>),
            Replication(P<'a, Replication>),
            RequireStrictness(P<'a, RequireStrictness>),
            SecretString(P<'a, SecretString>),
            SslKey(P<'a, SslKey>),
            SslMode(P<'a, SslMode>),
            SslNegotiation(P<'a, SslNegotiation>),
            SslProtocolVersion(P<'a, SslProtocolVersion>),
            SslRootCert(P<'a, SslRootCert>),
            String(P<'a, String>),
            TargetSessionAttrs(P<'a, TargetSessionAttrs>),
            U32(P<'a, u32>),
            U64(P<'a, u64>),
            UTF8Path(P<'a, UTF8Path>),
        }

        impl Param<'_> {
            fn set(self, key: ParameterKey, value: &str) -> Result<(), ParametersFromStrError> {
                use Param as P;
                use ParamFromStrError as E;
                match self {
                    P::AllowStrictness(p) => Self::f(p, key, value, E::AllowStrictness),
                    P::AuthMethod(p) => Self::f(p, key, value, E::AuthMethod),
                    P::Bool(p) => Self::f(p, key, value, E::Bool),
                    P::ClientEncoding(p) => Self::f(p, key, value, E::ClientEncoding),
                    P::GssLib(p) => Self::f(p, key, value, E::GssLib),
                    P::HttpsUrl(p) => Self::f(p, key, value, E::HttpsUrl),
                    P::Key(p) => Self::f(p, key, value, E::Key),
                    P::LoadBalanceHosts(p) => Self::f(p, key, value, E::LoadBalanceHosts),
                    P::NonEmptyString(p) => Self::f(p, key, value, E::NonEmptyString),
                    P::OAuthScope(p) => Self::f(p, key, value, E::OAuthScope),
                    P::Options(p) => Self::f(p, key, value, E::Options),
                    P::ProtocolVersion(p) => Self::f(p, key, value, E::ProtocolVersion),
                    P::Replication(p) => Self::f(p, key, value, E::Replication),
                    P::RequireStrictness(p) => Self::f(p, key, value, E::RequireStrictness),
                    P::SecretString(p) => Self::f(p, key, value, E::SecretString),
                    P::SslKey(p) => Self::f(p, key, value, E::SslKey),
                    P::SslMode(p) => Self::f(p, key, value, E::SslMode),
                    P::SslNegotiation(p) => Self::f(p, key, value, E::SslNegotiation),
                    P::SslProtocolVersion(p) => Self::f(p, key, value, E::SslProtocolVersion),
                    P::SslRootCert(p) => Self::f(p, key, value, E::SslRootCert),
                    P::String(p) => Self::f(p, key, value, E::String),
                    P::TargetSessionAttrs(p) => Self::f(p, key, value, E::TargetSessionAttrs),
                    P::U32(p) => Self::f(p, key, value, E::U32),
                    P::U64(p) => Self::f(p, key, value, E::U64),
                    P::UTF8Path(p) => Self::f(p, key, value, E::UTF8Path),
                }
            }

            fn f<T, E>(
                param: &mut Option<T>,
                key: ParameterKey,
                value: &str,
                err: impl Fn(E) -> ParamFromStrError,
            ) -> Result<(), ParametersFromStrError>
            where
                T: FromStr<Err = E>,
            {
                if param.is_some() {
                    Err(ParametersFromStrError::Duplicate {
                        key,
                        value: value.to_owned(),
                    })
                } else {
                    *param = Some(
                        value
                            .parse()
                            .map_err(|e| ParametersFromStrError::Param(err(e)))?,
                    );
                    Ok(())
                }
            }
        }

        let mut p = Parameters::builder().build();

        for pair in s.split('&') {
            use ParameterKey as K;

            let Some((key, value)) = pair.split_once('=') else {
                return Err(E::InvalidKeyValuePair(pair.to_owned()));
            };
            let value = urlencoding::decode(value).map_err(E::Decode)?;
            let value = value.as_ref();

            let key = key.parse().map_err(E::InvalidKey)?;

            let param: Param = match key {
                K::Passfile => (&mut p.passfile).into(),
                K::RequireAuth => (&mut p.require_auth).into(),
                K::ChannelBinding => (&mut p.channel_binding).into(),
                K::ConnectTimeout => (&mut p.connect_timeout).into(),
                K::ClientEncoding => (&mut p.client_encoding).into(),
                K::Options => (&mut p.options).into(),
                K::ApplicationName => (&mut p.application_name).into(),
                K::FallbackApplicationName => (&mut p.fallback_application_name).into(),
                K::Keepalives => (&mut p.keepalives).into(),
                K::KeepalivesIdle => (&mut p.keepalives_idle).into(),
                K::KeepalivesInterval => (&mut p.keepalives_interval).into(),
                K::KeepalivesCount => (&mut p.keepalives_count).into(),
                K::TcpUserTimeout => (&mut p.tcp_user_timeout).into(),
                K::Replication => (&mut p.replication).into(),
                K::Gssencmode => (&mut p.gss_enc_mode).into(),
                K::Sslmode => (&mut p.ssl_mode).into(),
                K::Requiressl => (&mut p.require_ssl).into(),
                K::Sslnegotiation => (&mut p.ssl_negotiation).into(),
                K::Sslcompression => (&mut p.ssl_compression).into(),
                K::Sslcert => (&mut p.ssl_cert).into(),
                K::Sslkey => (&mut p.ssl_key).into(),
                K::Sslkeylogfile => (&mut p.ssl_key_log_file).into(),
                K::Sslpassword => (&mut p.ssl_password).into(),
                K::Sslcertmode => (&mut p.ssl_cert_mode).into(),
                K::Sslrootcert => (&mut p.ssl_root_cert).into(),
                K::Sslcrl => (&mut p.ssl_crl).into(),
                K::Sslcrldir => (&mut p.ssl_crl_dir).into(),
                K::Sslsni => (&mut p.ssl_sni).into(),
                K::Requirepeer => (&mut p.require_peer).into(),
                K::SslMinProtocolVersion => (&mut p.ssl_min_protocol_version).into(),
                K::SslMaxProtocolVersion => (&mut p.ssl_max_protocol_version).into(),
                K::MinProtocolVersion => (&mut p.min_protocol_version).into(),
                K::MaxProtocolVersion => (&mut p.max_protocol_version).into(),
                K::Krbsrvname => (&mut p.krb_srv_name).into(),
                K::Gsslib => (&mut p.gss_lib).into(),
                K::Gssdelegation => (&mut p.gss_delegation).into(),
                K::ScramClientKey => (&mut p.scram_client_key).into(),
                K::ScramServerKey => (&mut p.scram_server_key).into(),
                K::Service => (&mut p.service).into(),
                K::TargetSessionAttrs => (&mut p.target_session_attrs).into(),
                K::LoadBalanceHosts => (&mut p.load_balance_hosts).into(),
                K::OauthIssuer => (&mut p.oauth_issuer).into(),
                K::OauthClientId => (&mut p.oauth_client_id).into(),
                K::OauthClientSecret => (&mut p.oauth_client_secret).into(),
                K::OauthScope => (&mut p.oauth_scope).into(),
            };

            param.set(key, value)?;
        }

        Ok(p)
    }
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug)]
pub enum ParametersFromStrError {
    Decode(FromUtf8Error),
    InvalidKeyValuePair(String),
    InvalidKey(<ParameterKey as FromStr>::Err),
    InvalidValue(String),
    Duplicate { key: ParameterKey, value: String },
    Param(ParamFromStrError),
}

#[derive(Debug)]
pub enum ParamFromStrError {
    AllowStrictness(<AllowStrictness as FromStr>::Err),
    AuthMethod(<AuthMethod as FromStr>::Err),
    Bool(<bool as FromStr>::Err),
    ClientEncoding(<ClientEncoding as FromStr>::Err),
    GssLib(<GssLib as FromStr>::Err),
    HttpsUrl(<HttpsUrl as FromStr>::Err),
    Key(<Key as FromStr>::Err),
    LoadBalanceHosts(<LoadBalanceHosts as FromStr>::Err),
    NonEmptyString(<NonEmptyString as FromStr>::Err),
    OAuthScope(<OAuthScope as FromStr>::Err),
    Options(<Options as FromStr>::Err),
    ProtocolVersion(<ProtocolVersion as FromStr>::Err),
    Replication(<Replication as FromStr>::Err),
    RequireStrictness(<RequireStrictness as FromStr>::Err),
    SecretString(<SecretString as FromStr>::Err),
    SslKey(<SslKey as FromStr>::Err),
    SslMode(<SslMode as FromStr>::Err),
    SslNegotiation(<SslNegotiation as FromStr>::Err),
    SslProtocolVersion(<SslProtocolVersion as FromStr>::Err),
    SslRootCert(<SslRootCert as FromStr>::Err),
    String(<String as FromStr>::Err),
    TargetSessionAttrs(<TargetSessionAttrs as FromStr>::Err),
    U32(<u32 as FromStr>::Err),
    U64(<u64 as FromStr>::Err),
    UTF8Path(<UTF8Path as FromStr>::Err),
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::{
        super::{
            param::{
                AllowStrictness, AuthMethod, ClientEncoding, GssLib, LoadBalanceHosts,
                ProtocolVersion, Replication, RequireStrictness, SslMode, SslNegotiation,
                SslProtocolVersion, TargetSessionAttrs,
            },
            tests::test_serdes,
        },
        ParameterKey, Parameters,
    };

    #[test]
    fn key_serdes() {
        test_serdes::<ParameterKey>([
            "passfile",
            "require_auth",
            "channel_binding",
            "connect_timeout",
            "client_encoding",
            "options",
            "application_name",
            "fallback_application_name",
            "keepalives",
            "keepalives_idle",
            "keepalives_interval",
            "tcp_user_timeout",
            "replication",
            "gssencmode",
            "sslmode",
            "requiressl",
            "sslnegotiation",
            "sslcompression",
            "sslcert",
            "sslkey",
            "sslkeylogfile",
            "sslpassword",
            "sslcertmode",
            "sslrootcert",
            "sslcrl",
            "sslcrldir",
            "sslsni",
            "requirepeer",
            "ssl_min_protocol_version",
            "ssl_max_protocol_version",
            "min_protocol_version",
            "max_protocol_version",
            "krbsrvname",
            "gsslib",
            "gssdelegation",
            "scram_client_key",
            "scram_server_key",
            "service",
            "target_session_attrs",
            "load_balance_hosts",
            "oauth_issuer",
            "oauth_client_id",
            "oauth_client_secret",
            "oauth_scope",
        ]);
    }

    #[test]
    fn empty() {
        assert!(Parameters::builder().build().to_string().is_empty());
    }

    #[test]
    fn all_params() {
        const STR: &str = "passfile=%2F&require_auth=gss&channel_binding=disable&connect_timeout=0&client_encoding=auto&options=-c%20geqo%3Doff&application_name=app&fallback_application_name=fallback&keepalives=true&keepalives_idle=0&keepalives_interval=0&keepalives_count=0&tcp_user_timeout=0&replication=database&gssencmode=disable&sslmode=allow&requiressl=true&sslnegotiation=direct&sslcompression=true&sslcert=.%2Fcert&sslkey=.%2Fkey&sslkeylogfile=.%2Flog&sslpassword=pass&sslcertmode=allow&sslrootcert=.%2Fcert&sslcrl=.%2Fcrl&sslcrldir=.%2Fdir&sslsni=true&requirepeer=peer&ssl_min_protocol_version=TLSv1&ssl_max_protocol_version=TLSv1.1&min_protocol_version=3.0&max_protocol_version=3.2&krbsrvname=krb&gsslib=gssapi&gssdelegation=true&scram_client_key=&scram_server_key=&service=service&target_session_attrs=any&load_balance_hosts=disable&oauth_issuer=https%3A%2F%2Fwww.wikipedia.org%2F&oauth_client_id=id&oauth_client_secret=secret&oauth_scope=s1%20s2";

        let params = Parameters::builder()
            .passfile("/".parse().unwrap())
            .require_auth(AuthMethod::Gss)
            .channel_binding(RequireStrictness::Disable)
            .connect_timeout(0)
            .client_encoding(ClientEncoding::Auto)
            .options("-c geqo=off".parse().unwrap())
            .application_name("app".to_owned())
            .fallback_application_name("fallback".to_owned())
            .keepalives(true)
            .keepalives_idle(0)
            .keepalives_interval(0)
            .keepalives_count(0)
            .tcp_user_timeout(0)
            .replication(Replication::Database)
            .gss_enc_mode(RequireStrictness::Disable)
            .ssl_mode(SslMode::Allow)
            .require_ssl(true)
            .ssl_negotiation(SslNegotiation::Direct)
            .ssl_compression(true)
            .ssl_cert("./cert".parse().unwrap())
            .ssl_key("./key".parse().unwrap())
            .ssl_key_log_file("./log".parse().unwrap())
            .ssl_password("pass".parse().unwrap())
            .ssl_cert_mode(AllowStrictness::Allow)
            .ssl_root_cert("./cert".parse().unwrap())
            .ssl_crl("./crl".parse().unwrap())
            .ssl_crl_dir("./dir".parse().unwrap())
            .ssl_sni(true)
            .require_peer("peer".to_owned())
            .ssl_min_protocol_version(SslProtocolVersion::TLSv1)
            .ssl_max_protocol_version(SslProtocolVersion::TLSv1_1)
            .min_protocol_version(ProtocolVersion::V3_0)
            .max_protocol_version(ProtocolVersion::V3_2)
            .krb_srv_name("krb".to_owned())
            .gss_lib(GssLib::Gssapi)
            .gss_delegation(true)
            .scram_client_key("".parse().unwrap())
            .scram_server_key("".parse().unwrap())
            .service("service".to_owned())
            .target_session_attrs(TargetSessionAttrs::Any)
            .load_balance_hosts(LoadBalanceHosts::Disable)
            .oauth_issuer("https://www.wikipedia.org".parse().unwrap())
            .oauth_client_id("id".to_owned())
            .oauth_client_secret("secret".parse().unwrap())
            .oauth_scope("s1 s2".parse().unwrap())
            .build();

        assert_eq!(STR.parse::<Parameters>().unwrap(), params);
        assert_eq!(params.to_string(), format!("?{STR}"));
    }

    #[test]
    fn display_only_encodes_values_not_structure() {
        let params = Parameters::builder()
            .options("-c geqo=off".parse().unwrap())
            .application_name("app".to_owned())
            .build();

        let serialized = params.to_string();

        assert_eq!(serialized, "?options=-c%20geqo%3Doff&application_name=app");

        let reparsed: Parameters = serialized.trim_start_matches('?').parse().unwrap();
        assert_eq!(reparsed, params);
    }
}
