use std::str::FromStr;

use derive_more::Display;

#[derive(Debug, Display, Eq, PartialEq)]
#[display(rename_all = "kebab-case")]
pub enum AuthMethod {
    Password,
    #[display("md5")]
    Md5,
    Gss,
    Sspi,
    #[display("scram-sha-256")]
    ScramSha256,
    Oauth,
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
