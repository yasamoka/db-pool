use std::str::FromStr;

use base64::{DecodeError, Engine, prelude::BASE64_STANDARD};
use derive_more::Display;

#[derive(Debug, Display, Eq, PartialEq)]
#[display("{}", BASE64_STANDARD.encode(_0))]
pub struct Key(Vec<u8>);

impl Key {
    pub fn new(key: Vec<u8>) -> Self {
        Self(key)
    }
}

impl FromStr for Key {
    type Err = DecodeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        BASE64_STANDARD.decode(s).map(Self)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::Key;

    #[test]
    fn valid() {
        const B64_STR: &str = "YWJj";
        const STR: &str = "abc";

        let actual = B64_STR.parse::<Key>().unwrap();
        let expected = Key(STR.as_bytes().to_owned());

        assert_eq!(actual, expected);
        assert_eq!(actual.to_string(), B64_STR);
    }

    #[test]
    fn invalid() {
        assert!("[".parse::<Key>().is_err());
    }
}
