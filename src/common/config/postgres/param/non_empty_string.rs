use std::str::FromStr;

use derive_more::Display;

#[derive(Debug, Display, Eq, PartialEq)]
pub struct NonEmptyString(String);

impl FromStr for NonEmptyString {
    type Err = NonEmptyStringFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            Err(NonEmptyStringFromStrError)
        } else {
            Ok(Self(s.to_owned()))
        }
    }
}

#[derive(Debug)]
pub struct NonEmptyStringFromStrError;

#[cfg(test)]
mod tests {
    use super::NonEmptyString;

    #[test]
    fn empty() {
        assert!("".parse::<NonEmptyString>().is_err());
    }

    #[test]
    fn non_empty() {
        const STR: &str = "abc";
        assert!(
            STR.parse::<NonEmptyString>()
                .is_ok_and(|s| s.to_string() == STR)
        );
    }
}
