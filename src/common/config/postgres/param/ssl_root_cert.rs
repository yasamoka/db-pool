use std::{convert::Infallible, path::PathBuf, str::FromStr};

use derive_more::Display;

#[derive(Debug, Display, Eq, PartialEq)]
pub enum SslRootCert {
    #[display("system")]
    System,
    #[display("{}", _0.to_string_lossy())]
    Specific(PathBuf),
}

impl FromStr for SslRootCert {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "system" => Self::System,
            s => Self::Specific(s.parse()?),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::SslRootCert;

    #[test]
    fn system() {
        const SYSTEM: &str = "system";
        let actual = SYSTEM.parse::<SslRootCert>().unwrap();
        assert_eq!(actual, SslRootCert::System);
        assert_eq!(actual.to_string(), SYSTEM);
    }

    #[test]
    fn specific() {
        const STR: &str = "abc";
        assert_eq!(
            STR.parse::<SslRootCert>().unwrap(),
            SslRootCert::Specific(Path::new(STR).to_owned())
        );
    }
}
