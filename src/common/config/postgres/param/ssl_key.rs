use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use derive_more::Display;

/// SSL secret key
#[derive(Debug, Display, Eq, PartialEq)]
pub enum SslKey {
    /// From path
    // TODO: ensure valid UTF-8
    #[display("{}", _0.to_string_lossy())]
    Path(PathBuf),
    /// Obtained from external engine
    #[display("{_0}")]
    ExternalEngine(ExternalEngine),
}

impl FromStr for SslKey {
    type Err = SslKeyFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with('/') || s.starts_with("./") {
            Ok(Self::Path(Path::new(s).to_path_buf()))
        } else if let Ok(external_engine) = s.parse() {
            Ok(Self::ExternalEngine(external_engine))
        } else {
            Err(SslKeyFromStrError(s.to_owned()))
        }
    }
}

#[derive(Debug)]
pub struct SslKeyFromStrError(pub String);

#[derive(Debug, Display, Eq, PartialEq)]
#[display("{engine_name}:{key_identifier}")]
pub struct ExternalEngine {
    engine_name: String,
    key_identifier: String,
}

impl FromStr for ExternalEngine {
    type Err = ExternalEngineFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.split(':');
        if let (Some(engine_name), Some(key_identifier), None) =
            (split.next(), split.next(), split.next())
        {
            Ok(Self {
                engine_name: engine_name.to_owned(),
                key_identifier: key_identifier.to_owned(),
            })
        } else {
            Err(ExternalEngineFromStrError(s.to_owned()))
        }
    }
}

#[derive(Debug)]
pub struct ExternalEngineFromStrError(pub String);

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::path::Path;

    use super::{ExternalEngine, SslKey};

    #[test]
    fn valid() {
        for (s, expected) in [
            ("/", SslKey::Path(Path::new("/").to_path_buf())),
            ("./", SslKey::Path(Path::new("./").to_path_buf())),
            (
                "name:identifier",
                SslKey::ExternalEngine(ExternalEngine {
                    engine_name: "name".to_owned(),
                    key_identifier: "identifier".to_owned(),
                }),
            ),
        ] {
            let actual = s.parse::<SslKey>().unwrap();
            assert_eq!(actual, expected);
            assert_eq!(actual.to_string(), s);
        }
    }

    #[test]
    fn invalid() {
        assert!("invalid".parse::<SslKey>().is_err());
    }
}
