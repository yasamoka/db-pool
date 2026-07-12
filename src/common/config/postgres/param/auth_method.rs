use std::str::FromStr;

use derive_more::Display;

/// Authentication method
#[derive(Debug, Display, Eq, PartialEq)]
#[display(rename_all = "kebab-case")]
pub enum AuthMethod {
    /// Plaintext password authentication
    Password,
    /// MD5 hashed password authentication
    #[display("md5")]
    Md5,
    /// Kerberos handshake via GSSAPI or GSS-encrypted channel
    Gss,
    /// Windows SSPI authentication
    Sspi,
    /// SCRAM-SHA-256 authentication exchange
    #[display("scram-sha-256")]
    ScramSha256,
    #[allow(clippy::doc_markdown)]
    /// OAuth bearer token
    Oauth,
    /// No prompt for authentication exchange
    None,
}

impl FromStr for AuthMethod {
    type Err = AuthMethodFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "password" => Ok(Self::Password),
            "md5" => Ok(Self::Md5),
            "gss" => Ok(Self::Gss),
            "sspi" => Ok(Self::Sspi),
            "scram-sha-256" => Ok(Self::ScramSha256),
            "oauth" => Ok(Self::Oauth),
            "none" => Ok(Self::None),
            s => Err(AuthMethodFromStrError(s.to_owned())),
        }
    }
}

#[derive(Debug)]
pub struct AuthMethodFromStrError(pub String);

#[cfg(test)]
mod tests {
    use super::{super::super::tests::test_serdes, AuthMethod};

    #[test]
    fn serdes() {
        test_serdes::<AuthMethod>([
            "password",
            "md5",
            "gss",
            "sspi",
            "scram-sha-256",
            "oauth",
            "none",
        ]);
    }
}
