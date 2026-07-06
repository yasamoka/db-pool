use std::{convert::Infallible, str::FromStr};

use derive_more::Display;

/// Client encoding
#[derive(Debug, Display, Eq, PartialEq)]
pub enum ClientEncoding {
    /// Determine right encoding from current client locale
    #[display("auto")]
    Auto,
    /// Specific encoding
    #[display("{_0}")]
    Specific(String),
}

impl FromStr for ClientEncoding {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "auto" => Self::Auto,
            s => Self::Specific(s.to_owned()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::ClientEncoding;

    #[test]
    fn auto() {
        const AUTO: &str = "auto";
        let actual = AUTO.parse::<ClientEncoding>().unwrap();
        assert_eq!(actual, ClientEncoding::Auto);
        assert_eq!(actual.to_string(), AUTO);
    }

    #[test]
    fn specific() {
        const STR: &str = "abc";
        assert_eq!(
            STR.parse::<ClientEncoding>().unwrap(),
            ClientEncoding::Specific(STR.to_owned())
        );
    }
}
