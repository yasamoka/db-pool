use std::{
    convert::Infallible,
    path::{Path, PathBuf},
    str::FromStr,
};

use derive_more::Display;

/// Path encoded in UTF-8
#[derive(Debug, Display, Eq, PartialEq)]
pub struct UTF8Path(String);

impl FromStr for UTF8Path {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Path::new(s).to_string_lossy().into_owned()))
    }
}

impl TryFrom<&Path> for UTF8Path {
    type Error = UTF8PathFromPathError;

    fn try_from(value: &Path) -> Result<Self, Self::Error> {
        use UTF8PathFromPathError as E;

        if let Some(value) = value.to_str() {
            Ok(Self(value.to_owned()))
        } else {
            Err(E::InvalidUTF8)
        }
    }
}

impl From<UTF8Path> for PathBuf {
    fn from(value: UTF8Path) -> Self {
        Path::new(value.0.as_str()).to_path_buf()
    }
}

#[derive(Debug)]
pub enum UTF8PathFromPathError {
    InvalidUTF8,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::UTF8Path;

    #[test]
    fn from_path() {
        for input in ["/", "./"] {
            assert!(
                input
                    .parse::<UTF8Path>()
                    .is_ok_and(|p| p.to_string() == input)
            );
        }
    }
}
