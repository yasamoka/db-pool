use std::{convert::Infallible, str::FromStr};

use derive_more::Display;

#[allow(clippy::doc_markdown)]
/// OAuth access request scope
#[derive(Debug, Display, Eq, PartialEq)]
#[display("{}", _0.join(" "))]
pub struct OAuthScope(pub Vec<String>);

impl FromStr for OAuthScope {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(
            s.split(' ')
                .filter_map(|s| {
                    if s.is_empty() {
                        None
                    } else {
                        Some(s.to_owned())
                    }
                })
                .collect(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::OAuthScope;

    #[test]
    fn test() {
        for (input, vec, expected) in [
            ("", Vec::<String>::new(), ""),
            ("s1", vec!["s1".to_owned()], "s1"),
            (
                "s1 s2 s3",
                vec!["s1".to_owned(), "s2".to_owned(), "s3".to_owned()],
                "s1 s2 s3",
            ),
            (
                " s1  s2 s3    ",
                vec!["s1".to_owned(), "s2".to_owned(), "s3".to_owned()],
                "s1 s2 s3",
            ),
        ] {
            assert!(
                input
                    .parse::<OAuthScope>()
                    .is_ok_and(|scope| scope.0 == vec && scope.to_string() == expected)
            );
        }
    }
}
