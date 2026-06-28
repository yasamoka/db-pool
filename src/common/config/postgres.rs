use std::{
    borrow::Cow,
    collections::HashMap,
    env::{VarError, var},
    ffi::OsString,
    fmt::{Display, Write},
    path::{Path, PathBuf, absolute},
    str::FromStr,
};

use base64::{Engine, engine::general_purpose::STANDARD};
use bon::{Builder, bon};
use derive_more::{Display, Into};
use secrecy::{ExposeSecret, SecretString};
use url::Url;
use urlencoding::encode;

use super::credentials::Credentials;

const PREFIX: &str = "postgres";
const DEFAULT_USERNAME: &str = "postgres";
const USERNAME_ENV_VAR: &str = "POSTGRES_USERNAME";
const PASSWORD_ENV_VAR: &str = "POSTGRES_PASSWORD";
const HOST_ENV_VAR: &str = "POSTGRES_HOST";

/// Privileged Postgres configuration
pub struct PrivilegedPostgresConfig {
    pub(crate) credentials: Credentials<'static>,
    pub(crate) host: PostgresHostConfigInner,
    parameters: Parameters,
}

#[bon]
impl PrivilegedPostgresConfig {
    /// Build privileged Postgres configuration
    #[builder]
    pub fn new(
        #[builder(default = DEFAULT_USERNAME.to_owned())] username: String,
        password: Option<String>,
        #[builder(
        default = PostgresHostConfigInner::default(),
        with = |host: PostgresHostConfig| -> Result<
            _,
            <PostgresHostConfigInner as TryFrom<PostgresHostConfig>>::Error,
        > { PostgresHostConfigInner::try_from(host) },
    )]
        host: PostgresHostConfigInner,
        #[builder(default = Parameters::builder().build())] parameters: Parameters,
    ) -> Self {
        Self {
            credentials: Credentials::new(username, password),
            host,
            parameters,
        }
    }

    /// Creates a new privileged Postgres configuration from environment variables
    /// # Environment variables
    /// - `POSTGRES_USERNAME`
    /// - `POSTGRES_PASSWORD`
    /// - `POSTGRES_HOST`
    /// # Defaults
    /// - Username: postgres
    /// - Password: {blank}
    /// - Host: localhost:5432
    pub fn from_env() -> Result<Self, FromEnvError> {
        use FromEnvError as E;

        let username = var(USERNAME_ENV_VAR).unwrap_or(DEFAULT_USERNAME.to_owned());
        let password = var(PASSWORD_ENV_VAR).ok();

        let host = match var(HOST_ENV_VAR) {
            Ok(host) => host.parse().map_err(E::HostConfig),
            Err(VarError::NotPresent) => Ok(PostgresHostConfigInner::default()),
            Err(VarError::NotUnicode(s)) => Err(E::HostIsNotUnicode(s)),
        }?;

        Ok(Self {
            credentials: Credentials::new(username, password),
            host,
            parameters: Parameters::builder().build(),
        })
    }

    pub(crate) fn default_connection_url(&self) -> String {
        let Self {
            credentials,
            host,
            parameters,
        } = self;

        format!("{PREFIX}://{credentials}@{host}{parameters}")
    }

    pub(crate) fn privileged_database_connection_url(&self, db_name: &str) -> String {
        let Self {
            credentials,
            host,
            parameters,
        } = self;

        format!("{PREFIX}://{credentials}@{host}/{db_name}{parameters}")
    }

    pub(crate) fn restricted_database_connection_url(
        &self,
        username: &str,
        password: Option<&str>,
        db_name: &str,
    ) -> String {
        format!(
            "{}://{}@{}/{}",
            PREFIX,
            Credentials::new(username, password),
            self.host,
            db_name
        )
    }
}

#[derive(Debug)]
pub enum FromEnvError {
    HostIsNotUnicode(OsString),
    HostConfig(PostgresHostConfigFromStrError),
}

pub enum PostgresHostConfig {
    TcpIp { host: String, port: u16 },
    UnixSocket(PathBuf),
}

#[derive(Clone, Debug, Display)]
pub enum PostgresHostConfigInner {
    #[display("{host}:{port}")]
    TcpIp { host: String, port: u16 },
    #[display("{}", encode(socket))]
    UnixSocket { socket: String },
}

impl Default for PostgresHostConfigInner {
    fn default() -> Self {
        Self::TcpIp {
            host: "localhost".to_owned(),
            port: 5432,
        }
    }
}

impl FromStr for PostgresHostConfigInner {
    type Err = PostgresHostConfigFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use PostgresHostConfigFromStrError as E;

        if s.starts_with('/') || s.starts_with("./") {
            Ok(Self::UnixSocket {
                socket: s.to_owned(),
            })
        } else if let Some((host, port)) = s.split_once(':') {
            port.parse()
                .map(|port| Self::TcpIp {
                    host: host.to_owned(),
                    port,
                })
                .map_err(E::InvalidPort)
        } else {
            Err(E::InvalidHost(s.to_owned()))
        }
    }
}

#[derive(Debug)]
pub enum PostgresHostConfigFromStrError {
    HostIsNotUnicode(OsString),
    InvalidHost(String),
    InvalidPort(std::num::ParseIntError),
}

impl TryFrom<PostgresHostConfig> for PostgresHostConfigInner {
    type Error = TryFromPostgresHostConfigError;

    fn try_from(value: PostgresHostConfig) -> Result<Self, Self::Error> {
        use TryFromPostgresHostConfigError as E;

        Ok(match value {
            PostgresHostConfig::TcpIp { host, port } => {
                PostgresHostConfigInner::TcpIp { host, port }
            }
            PostgresHostConfig::UnixSocket(socket) => PostgresHostConfigInner::UnixSocket {
                socket: absolute(socket)
                    .map_err(E::Path)?
                    .to_str()
                    .ok_or(E::UnixSocketAddressIsNotUTF8)?
                    .to_owned(),
            },
        })
    }
}

pub enum TryFromPostgresHostConfigError {
    Path(std::io::Error),
    UnixSocketAddressIsNotUTF8,
}

#[derive(Builder)]
pub struct Parameters {
    passfile: Option<UTF8Path>,
    require_auth: Option<AuthMethod>,
    channel_binding: Option<RequireStrictness>,
    connect_timeout: Option<u64>,
    client_encoding: Option<ClientEncoding>,
    options: Option<Options>,
    application_name: Option<String>,
    fallback_application_name: Option<String>,
    keepalives: Option<bool>,
    keepalives_idle: Option<u64>,
    keepalives_interval: Option<u64>,
    keepalives_count: Option<u32>,
    tcp_user_timeout: Option<u64>,
    replication: Option<Replication>,
    gss_enc_mode: Option<RequireStrictness>,
    ssl_mode: Option<SslMode>,
    require_ssl: Option<bool>,
    ssl_negotiation: Option<SslNegotiation>,
    ssl_compression: Option<bool>,
    ssl_cert: Option<UTF8Path>,
    ssl_key: Option<SslKey>,
    ssl_key_log_file: Option<UTF8Path>,
    ssl_password: Option<NonEmptyString>,
    ssl_cert_mode: Option<AllowStrictness>,
    ssl_root_cert: Option<SslRootCert>,
    ssl_crl: Option<UTF8Path>,
    ssl_crl_dir: Option<UTF8Path>,
    ssl_sni: Option<bool>,
    require_peer: Option<String>,
    ssl_min_protocol_version: Option<SslProtocolVersion>,
    ssl_max_protocol_version: Option<SslProtocolVersion>,
    min_protocol_version: Option<ProtocolVersion>,
    max_protocol_version: Option<ProtocolVersion>,
    krb_srv_name: Option<String>,
    gss_lib: Option<GssLib>,
    gss_delegation: Option<bool>,
    scram_client_key: Option<Key>,
    scram_server_key: Option<Key>,
    service: Option<String>,
    target_session_attrs: Option<TargetSessionAttrs>,
    load_balance_hosts: Option<LoadBalanceHosts>,
    oauth_issuer: Option<HttpsUrl>,
    oauth_client_id: Option<String>,
    oauth_client_secret: Option<SecretString>,
    oauth_scope: Option<OAuthScope>,
}

impl Parameters {
    const PASSFILE_KEY: &str = "passfile";
    const REQUIRE_AUTH_KEY: &str = "require_auth";
    const CHANNEL_BINDING_KEY: &str = "channel_binding";
    const CONNECT_TIMEOUT_KEY: &str = "connect_timeout";
    const CLIENT_ENCODING_KEY: &str = "client_encoding";
    const OPTIONS_KEY: &str = "options";
    const APPLICATION_NAME_KEY: &str = "application_name";
    const FALLBACK_APPLICATION_NAME_KEY: &str = "fallback_application_name";
    const KEEPALIVES_KEY: &str = "keepalives";
    const KEEPALIVES_IDLE_KEY: &str = "keepalives_idle";
    const KEEPALIVES_INTERVAL_KEY: &str = "keepalives_interval";
    const KEEPALIVES_COUNT_KEY: &str = "keepalives_count";
    const TCP_USER_TIMEOUT_KEY: &str = "tcp_user_timeout";
    const REPLICATION_KEY: &str = "replication";
    const GSS_ENC_MODE_KEY: &str = "gssencmode";
    const SSL_MODE_KEY: &str = "sslmode";
    const REQUIRE_SSL_KEY: &str = "requiressl";
    const SSL_NEGOTIATION_KEY: &str = "sslnegotiation";
    const SSL_COMPRESSION_KEY: &str = "sslcompression";
    const SSL_CERT_KEY: &str = "sslcert";
    const SSL_KEY_KEY: &str = "sslkey";
    const SSL_KEY_LOG_FILE_KEY: &str = "sslkeylogfile";
    const SSL_PASSWORD_KEY: &str = "sslpassword";
    const SSL_CERT_MODE_KEY: &str = "sslcertmode";
    const SSL_ROOT_CERT_KEY: &str = "sslrootcert";
    const SSL_CRL_KEY: &str = "sslcrl";
    const SSL_CRL_DIR_KEY: &str = "sslcrldir";
    const SSL_SNI_KEY: &str = "sslsni";
    const REQUIRE_PEER_KEY: &str = "requirepeer";
    const SSL_MIN_PROTOCOL_VERSION_KEY: &str = "ssl_min_protocol_version";
    const SSL_MAX_PROTOCOL_VERSION_KEY: &str = "ssl_max_protocol_version";
    const MIN_PROTOCOL_VERSION_KEY: &str = "min_protocol_version";
    const MAX_PROTOCOL_VERSION_KEY: &str = "max_protocol_version";
    const KRB_SRV_NAME_KEY: &str = "krbsrvname";
    const GSS_LIB_KEY: &str = "gsslib";
    const GSS_DELEGATION_KEY: &str = "gssdelegation";
    const SCRAM_CLIENT_KEY_KEY: &str = "scram_client_key";
    const SCRAM_SERVER_KEY_KEY: &str = "scram_server_key";
    const SERVICE_KEY: &str = "service";
    const TARGET_SESSION_ATTRS_KEY: &str = "target_session_attrs";
    const LOAD_BALANCE_HOSTS_KEY: &str = "load_balance_hosts";
    const OAUTH_ISSUER_KEY: &str = "oauth_issuer";
    const OAUTH_CLIENT_ID_KEY: &str = "oauth_client_id";
    const OAUTH_CLIENT_SECRET_KEY: &str = "oauth_client_secret";
    const OAUTH_SCOPE_KEY: &str = "oauth_scope";
}

impl Display for Parameters {
    #[allow(clippy::too_many_lines)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
            (Self::PASSFILE_KEY, v(passfile)),
            (Self::REQUIRE_AUTH_KEY, v(require_auth)),
            (Self::CHANNEL_BINDING_KEY, v(channel_binding)),
            (Self::CONNECT_TIMEOUT_KEY, v(connect_timeout)),
            (Self::CLIENT_ENCODING_KEY, v(client_encoding)),
            (Self::OPTIONS_KEY, v(options)),
            (Self::APPLICATION_NAME_KEY, v(application_name)),
            (
                Self::FALLBACK_APPLICATION_NAME_KEY,
                v(fallback_application_name),
            ),
            (Self::KEEPALIVES_KEY, v(keepalives)),
            (Self::KEEPALIVES_IDLE_KEY, v(keepalives_idle)),
            (Self::KEEPALIVES_INTERVAL_KEY, v(keepalives_interval)),
            (Self::KEEPALIVES_COUNT_KEY, v(keepalives_count)),
            (Self::TCP_USER_TIMEOUT_KEY, v(tcp_user_timeout)),
            (Self::REPLICATION_KEY, v(replication)),
            (Self::GSS_ENC_MODE_KEY, v(gss_enc_mode)),
            (Self::SSL_MODE_KEY, v(ssl_mode)),
            (Self::REQUIRE_SSL_KEY, v(require_ssl)),
            (Self::SSL_NEGOTIATION_KEY, v(ssl_negotiation)),
            (Self::SSL_COMPRESSION_KEY, v(ssl_compression)),
            (Self::SSL_CERT_KEY, v(ssl_cert)),
            (Self::SSL_KEY_KEY, v(ssl_key)),
            (Self::SSL_KEY_LOG_FILE_KEY, v(ssl_key_log_file)),
            (Self::SSL_PASSWORD_KEY, v(ssl_password)),
            (Self::SSL_CERT_MODE_KEY, v(ssl_cert_mode)),
            (Self::SSL_ROOT_CERT_KEY, v(ssl_root_cert)),
            (Self::SSL_CRL_KEY, v(ssl_crl)),
            (Self::SSL_CRL_DIR_KEY, v(ssl_crl_dir)),
            (Self::SSL_SNI_KEY, v(ssl_sni)),
            (Self::REQUIRE_PEER_KEY, v(require_peer)),
            (
                Self::SSL_MIN_PROTOCOL_VERSION_KEY,
                v(ssl_min_protocol_version),
            ),
            (
                Self::SSL_MAX_PROTOCOL_VERSION_KEY,
                v(ssl_max_protocol_version),
            ),
            (Self::MIN_PROTOCOL_VERSION_KEY, v(min_protocol_version)),
            (Self::MAX_PROTOCOL_VERSION_KEY, v(max_protocol_version)),
            (Self::KRB_SRV_NAME_KEY, v(krb_srv_name)),
            (Self::GSS_LIB_KEY, v(gss_lib)),
            (Self::GSS_DELEGATION_KEY, v(gss_delegation)),
            (Self::SCRAM_CLIENT_KEY_KEY, v(scram_client_key)),
            (Self::SCRAM_SERVER_KEY_KEY, v(scram_server_key)),
            (Self::SERVICE_KEY, v(service)),
            (Self::TARGET_SESSION_ATTRS_KEY, v(target_session_attrs)),
            (Self::LOAD_BALANCE_HOSTS_KEY, v(load_balance_hosts)),
            (Self::OAUTH_ISSUER_KEY, v(oauth_issuer)),
            (Self::OAUTH_CLIENT_ID_KEY, v(oauth_client_id)),
            (
                Self::OAUTH_CLIENT_SECRET_KEY,
                v(&oauth_client_secret
                    .as_ref()
                    .map(ExposeSecret::expose_secret)),
            ),
            (Self::OAUTH_SCOPE_KEY, v(oauth_scope)),
        ]
        .into_iter()
        .filter_map(|(key, value)| value.map(|value| format!("{key}={value}")))
        .collect::<Vec<_>>()
        .join("&");

        if !data.is_empty() {
            f.write_char('?')?;
            f.write_str(&encode(data.as_str()))?;
        }

        Ok(())
    }
}

impl FromStr for Parameters {
    type Err = ParametersFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use ParametersFromStrError as E;

        let b = Parameters::builder().build();

        for pair in s.split('&') {
            let mut split = pair.split('=');
            let (Some(key), Some(value), None) = (split.next(), split.next(), split.next()) else {
                return Err(E::InvalidKeyValuePair(pair.to_owned()));
            };
            match key {
                Self::PASSFILE_KEY => {b.passfile = value.parse();},
                Self::REQUIRE_AUTH_KEY => {b.require_auth = value.parse();},
                Self::CHANNEL_BINDING_KEY => {b.channel_binding = value.parse();},
                Self::CONNECT_TIMEOUT_KEY => {b.connect_timeout = value.parse();},
                Self::CLIENT_ENCODING_KEY => {b.client_encoding = value.parse();},
                Self::OPTIONS_KEY => {b.options = value.parse();},
                Self::APPLICATION_NAME_KEY => {b.application_name = value.parse();},
                Self::FALLBACK_APPLICATION_NAME_KEY => {b.fallback_application_name = value.parse();},
                Self::KEEPALIVES_KEY => {b.keepalives = value.parse();},
                Self::KEEPALIVES_IDLE_KEY => {b.keepalives_idle = value.parse();},
                Self::KEEPALIVES_INTERVAL_KEY => {b.keepalives_interval = value.parse();},
                Self::KEEPALIVES_COUNT_KEY => {b.keepalives_count = value.parse();},
                Self::TCP_USER_TIMEOUT_KEY => {b.tcp_user_timeout = value.parse();},
                Self::REPLICATION_KEY => {b.replication = value.parse();},
                Self::GSS_ENC_MODE_KEY => {b.gss_enc_mode = value.parse();},
                Self::SSL_MODE_KEY => {b.ssl_mode = value.parse();},
                Self::REQUIRE_SSL_KEY => {b.require_ssl = value.parse();},
                Self::SSL_NEGOTIATION_KEY => {b.ssl_negotiation = value.parse();},
                Self::SSL_COMPRESSION_KEY => {b.ssl_compression = value.parse();},
                Self::SSL_CERT_KEY => {b.ssl_cert = value.parse();},
                Self::SSL_KEY_KEY => {b.ssl_key = value.parse();},
                Self::SSL_KEY_LOG_FILE_KEY => {b.ssl_key_log_file = value.parse();},
                Self::SSL_PASSWORD_KEY => {b.ssl_password = value.parse();},
                Self::SSL_CERT_MODE_KEY => {b.ssl_cert_mode = value.parse();},
                Self::SSL_ROOT_CERT_KEY => {b.ssl_root_cert = value.parse();},
                Self::SSL_CRL_KEY => {b.ssl_crl = value.parse();},
                Self::SSL_CRL_DIR_KEY => {b.ssl_crl_dir = value.parse();},
                Self::SSL_SNI_KEY => {b.ssl_sni = value.parse();},
                Self::REQUIRE_PEER_KEY => {b.require_peer = value.parse();},
                Self::SSL_MIN_PROTOCOL_VERSION_KEY => {b.ssl_min_protocol_version = value.parse();},
                Self::SSL_MAX_PROTOCOL_VERSION_KEY => {b.ssl_max_protocol_version = value.parse();},
                Self::MIN_PROTOCOL_VERSION_KEY => {b.min_protocol_version = value.parse();},
                Self::MAX_PROTOCOL_VERSION_KEY => {b.max_protocol_version = value.parse();},
                Self::KRB_SRV_NAME_KEY => {b.krb_srv_name = value.parse();},
                Self::GSS_LIB_KEY => {b.gss_lib = value.parse();},
                Self::GSS_DELEGATION_KEY => {b.gss_delegation = value.parse();},
                Self::SCRAM_CLIENT_KEY_KEY => {b.scram_client_key = value.parse();},
                Self::SCRAM_SERVER_KEY_KEY => {b.scram_server_key = value.parse();},
                Self::SERVICE_KEY => {b.service = value.parse();},
                Self::TARGET_SESSION_ATTRS_KEY => {b.target_session_attrs = value.parse();},
                Self::LOAD_BALANCE_HOSTS_KEY => {b.load_balance_hosts = value.parse();},
                Self::OAUTH_ISSUER_KEY => {b.oauth_issuer = value.parse();},
                Self::OAUTH_CLIENT_ID_KEY => {b.oauth_client_id = value.parse();},
                Self::OAUTH_CLIENT_SECRET_KEY => {b.&oauth_client_secret.as_ref().map(ExposeSecret::expose_secret) = value.parse();},
                Self::OAUTH_SCOPE_KEY => {b.oauth_scope = value.parse();},
            }
        }

        Ok(b)
    }
}

#[allow(clippy::enum_variant_names)]
pub enum ParametersFromStrError {
    InvalidKeyValuePair(String),
    InvalidKey(String),
    InvalidValue(String),
}

#[derive(Display)]
#[display(rename_all = "kebab-case")]
pub enum RequireStrictness {
    Require,
    Prefer,
    Disable,
}

#[derive(Display)]
#[display(rename_all = "kebab-case")]
pub enum AllowStrictness {
    Require,
    Allow,
    Disable,
}

#[derive(Display)]
// TODO: try to remove
#[display("{_0}")]
pub struct UTF8Path(String);

impl TryFrom<&Path> for UTF8Path {
    type Error = UTF8PathFromPathError;

    fn try_from(value: &Path) -> Result<Self, Self::Error> {
        use UTF8PathFromPathError as E;

        if !value.is_absolute() {
            return Err(E::NotAbsolute);
        }

        if let Some(value) = value.to_str() {
            Ok(Self(value.to_owned()))
        } else {
            Err(E::InvalidUTF8)
        }
    }
}

impl From<UTF8Path> for PathBuf {
    fn from(value: UTF8Path) -> Self {
        Path::new(value.0.as_str()).to_path_buf()
    }
}

pub enum UTF8PathFromPathError {
    NotAbsolute,
    InvalidUTF8,
}

#[derive(Display)]
#[display("{_0}")]
pub struct NonEmptyString(String);

impl TryFrom<String> for NonEmptyString {
    type Error = NonEmptyStringFromStringError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() {
            Err(NonEmptyStringFromStringError)
        } else {
            Ok(Self(value))
        }
    }
}

pub struct NonEmptyStringFromStringError;

#[derive(Display)]
#[display(rename_all = "kebab-case")]
pub enum AuthMethod {
    Password,
    Md5,
    Gss,
    Sspi,
    #[display("scram-sha-256")]
    ScramSha256,
    #[display("oauth")]
    OAuth,
    None,
}

#[derive(Display)]
pub enum ClientEncoding {
    #[display("auto")]
    Auto,
    #[display("{_0}")]
    Specific(String),
}

#[derive(Into)]
pub struct Options(HashMap<Cow<'static, str>, Cow<'static, str>>);

impl Options {
    pub fn new<K, V>(options: impl IntoIterator<Item = (K, V)>) -> Self
    where
        K: Into<Cow<'static, str>>,
        V: Into<Cow<'static, str>>,
    {
        Self(
            options
                .into_iter()
                .map(|(key, value)| (key.into(), value.into()))
                .collect(),
        )
    }
}

impl Display for Options {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn escape(s: &str) -> String {
            s.replace('\\', r"\\").replace(' ', r"\ ")
        }

        let add = |f: &mut std::fmt::Formatter<'_>, key, value| {
            f.write_str("-c ")?;
            f.write_str(escape(key).as_str())?;
            f.write_char('=')?;
            f.write_str(escape(value).as_str())?;
            Ok::<_, std::fmt::Error>(())
        };

        let mut iter = self.0.iter();

        if let Some((key, value)) = iter.next() {
            add(f, key, value)?;
            for (key, value) in iter {
                f.write_char(' ')?;
                add(f, key, value)?;
            }
        }

        Ok(())
    }
}

#[derive(Display)]
#[display(rename_all = "kebab-case")]
pub enum Replication {
    True,
    Database,
    False,
}

#[derive(Display)]
#[display(rename_all = "kebab-case")]
pub enum SslMode {
    Disable,
    Allow,
    Prefer,
    Require,
    VerifyCa,
    VerifyFull,
}

#[derive(Display)]
#[display(rename_all = "kebab-case")]
pub enum SslNegotiation {
    Postgres,
    Direct,
}

#[derive(Display)]
pub enum SslKey {
    // ensure valid UTF-8
    #[display("{}", _0.to_string_lossy())]
    Path(PathBuf),
    #[display("{_0}")]
    ExternalEngine(ExternalEngine),
}

#[derive(Display)]
#[display("{engine_name}:{key_identifier}")]
pub struct ExternalEngine {
    engine_name: String,
    key_identifier: String,
}

#[derive(Display)]
pub enum SslRootCert {
    #[display("system")]
    System,
    #[display("{}", _0.to_string_lossy())]
    Specific(PathBuf),
}

#[derive(Display)]
pub enum SslProtocolVersion {
    #[display("TLSv1")]
    TLSv1,
    #[display("TLSv1.1")]
    TLSv1_1,
    #[display("TLSv1.2")]
    TLSv1_2,
    #[display("TLSv1.3")]
    TLSv1_3,
}

#[derive(Display)]
pub enum ProtocolVersion {
    #[display("3.0")]
    V3_0,
    #[display("3.2")]
    V3_2,
    #[display("latest")]
    Latest,
}

#[derive(Display)]
pub enum GssLib {
    #[display("gssapi")]
    GssApi,
}

#[derive(Display)]
#[display("{}", STANDARD.encode(_0))]
pub struct Key(Vec<u8>);

impl Key {
    pub fn new(key: Vec<u8>) -> Self {
        Self(key)
    }
}

#[derive(Display)]
#[display(rename_all = "kebab-case")]
pub enum TargetSessionAttrs {
    Any,
    ReadWrite,
    ReadOnly,
    Primary,
    Standby,
    PreferStandby,
}

#[derive(Display)]
#[display(rename_all = "kebab-case")]
pub enum LoadBalanceHosts {
    Disable,
    Random,
}

#[derive(Display)]
#[display("{_0}")]
pub struct HttpsUrl(Url);

impl TryFrom<Url> for HttpsUrl {
    type Error = HttpsUrlFromUrlError;

    fn try_from(value: Url) -> Result<Self, Self::Error> {
        if value.scheme() == "https" {
            Ok(Self(value))
        } else {
            Err(HttpsUrlFromUrlError)
        }
    }
}

pub struct HttpsUrlFromUrlError;

#[derive(Display)]
#[display("{}", _0.join(" "))]
pub struct OAuthScope(Vec<String>);

#[cfg(feature = "postgres")]
mod postgres_impl {
    use std::time::Duration;

    use super::{Parameters, PostgresHostConfigInner, PrivilegedPostgresConfig};

    impl From<PrivilegedPostgresConfig> for r2d2_postgres::postgres::Config {
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
                    config.host(host.as_str()).port(port);
                }
                PostgresHostConfigInner::UnixSocket { socket } => {
                    config.host(&socket);
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
                    config.options(options.to_string().as_str());
                }

                if let Some(application_name) = application_name {
                    config.application_name(application_name.as_str());
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
}

#[cfg(feature = "sqlx-postgres")]
mod sqlx_postgres_impl {
    use std::{collections::HashMap, path::PathBuf};

    use super::{
        Parameters, PostgresHostConfigInner, PrivilegedPostgresConfig, SslKey, SslMode, SslRootCert,
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
                PostgresHostConfigInner::TcpIp { host, port } => {
                    opts.host(host.as_str()).port(port)
                }
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
}

#[cfg(feature = "tokio-postgres")]
mod tokio_postgres_impl {
    use std::time::Duration;

    use super::{Parameters, PostgresHostConfigInner, PrivilegedPostgresConfig};

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
}

#[cfg(any(feature = "postgres", feature = "tokio-postgres"))]
mod tokio_postgres_config_impl {
    use super::{LoadBalanceHosts, RequireStrictness, SslMode, SslNegotiation, TargetSessionAttrs};

    impl From<RequireStrictness> for tokio_postgres::config::ChannelBinding {
        fn from(value: RequireStrictness) -> Self {
            use RequireStrictness as T;
            use tokio_postgres::config::ChannelBinding as U;
            match value {
                T::Disable => U::Disable,
                T::Require => U::Require,
                T::Prefer => U::Prefer,
            }
        }
    }

    impl From<SslMode> for Option<tokio_postgres::config::SslMode> {
        fn from(value: SslMode) -> Self {
            use SslMode as T;
            use tokio_postgres::config::SslMode as U;
            match value {
                T::Disable => Some(U::Disable),
                T::Prefer => Some(U::Prefer),
                T::Require => Some(U::Require),
                T::Allow | T::VerifyCa | T::VerifyFull => None,
            }
        }
    }

    impl From<SslNegotiation> for tokio_postgres::config::SslNegotiation {
        fn from(value: SslNegotiation) -> Self {
            use SslNegotiation as T;
            use tokio_postgres::config::SslNegotiation as U;
            match value {
                T::Direct => U::Direct,
                T::Postgres => U::Postgres,
            }
        }
    }

    impl From<TargetSessionAttrs> for Option<tokio_postgres::config::TargetSessionAttrs> {
        fn from(value: TargetSessionAttrs) -> Self {
            use TargetSessionAttrs as T;
            use tokio_postgres::config::TargetSessionAttrs as U;
            match value {
                T::Any => Some(U::Any),
                T::ReadOnly => Some(U::ReadOnly),
                T::ReadWrite => Some(U::ReadWrite),
                T::PreferStandby | T::Primary | T::Standby => None,
            }
        }
    }

    impl From<LoadBalanceHosts> for tokio_postgres::config::LoadBalanceHosts {
        fn from(value: LoadBalanceHosts) -> Self {
            use LoadBalanceHosts as T;
            use tokio_postgres::config::LoadBalanceHosts as U;
            match value {
                T::Disable => U::Disable,
                T::Random => U::Random,
            }
        }
    }
}
