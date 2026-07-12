use std::str::FromStr;

use derive_more::Display;
use url::{ParseError, Url};

/// HTTPS URL
#[derive(Debug, Display, Eq, PartialEq)]
pub struct HttpsUrl(Url);

impl FromStr for HttpsUrl {
    type Err = HttpsUrlFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use HttpsUrlFromStrError as E;

        s.parse::<Url>()
            .map_err(E::Parse)?
            .try_into()
            .map_err(|_| E::NotHttps)
    }
}

#[derive(Debug)]
pub enum HttpsUrlFromStrError {
    Parse(ParseError),
    NotHttps,
}

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

#[cfg(test)]
mod tests {
    use super::HttpsUrl;

    #[test]
    fn http() {
        assert!("http://www.wikipedia.org".parse::<HttpsUrl>().is_err());
    }

    #[test]
    fn https() {
        assert!("https://www.wikipedia.org".parse::<HttpsUrl>().is_ok());
    }

    #[test]
    fn other() {
        assert!("ftp://www.wikipedia.org".parse::<HttpsUrl>().is_err());
    }
}
