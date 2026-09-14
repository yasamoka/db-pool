mod config;
mod conversion;
mod host;
mod param;
mod params;

pub use config::{PrivilegedPostgresConfig, PrivilegedPostgresConfigBuilder};
pub use host::PostgresHostConfig;
pub use param::{
    AllowStrictness, AuthMethod, ClientEncoding, GssLib, HttpsUrl, Key, LoadBalanceHosts,
    NonEmptyString, OAuthScope, Options, ProtocolVersion, Replication, RequireStrictness,
    SecretString, SslKey, SslMode, SslNegotiation, SslProtocolVersion, SslRootCert,
    TargetSessionAttrs, UTF8Path,
};
pub use params::{Parameters, ParametersBuilder};

#[cfg(test)]
pub(super) mod tests {
    #![allow(clippy::unwrap_used)]

    use std::{fmt::Debug, str::FromStr};

    pub(super) fn test_serdes<T>(options: impl IntoIterator<Item = &'static str>)
    where
        T: FromStr + ToString,
        <T as FromStr>::Err: Debug,
    {
        for option in options {
            let e = option.parse::<T>().unwrap();
            let s = e.to_string();
            assert_eq!(s, option);
        }
    }
}
