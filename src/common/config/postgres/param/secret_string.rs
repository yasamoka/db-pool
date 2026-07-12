use std::{convert::Infallible, fmt::Display, str::FromStr};

use secrecy::ExposeSecret;

/// Secret string
#[derive(Debug)]
pub struct SecretString(secrecy::SecretString);

impl Eq for SecretString {}

impl PartialEq for SecretString {
    fn eq(&self, other: &Self) -> bool {
        self.0.expose_secret() == other.0.expose_secret()
    }
}

impl FromStr for SecretString {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(secrecy::SecretString::from(s)))
    }
}

impl Display for SecretString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.expose_secret())
    }
}

#[cfg(test)]
mod tests {
    use super::SecretString;

    #[test]
    fn serdes() {
        const STR: &str = "abc";
        assert!(
            STR.parse::<SecretString>()
                .is_ok_and(|s| s.to_string() == STR)
        );
    }
}
