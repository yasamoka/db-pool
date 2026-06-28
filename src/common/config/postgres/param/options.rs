use std::{
    borrow::Cow,
    collections::HashMap,
    fmt::{Display, Write},
    str::FromStr,
};

use derive_more::Into;
use regex::Regex;

#[derive(Debug, Eq, Into, PartialEq)]
pub struct Options(HashMap<Cow<'static, str>, Cow<'static, str>>);

impl Options {
    pub fn new<K, V>(options: impl IntoIterator<Item = (K, V)>) -> Self
    where
        K: Into<Cow<'static, str>>,
        V: Into<Cow<'static, str>>,
    {
        Self(
            options
                .into_iter()
                .map(|(key, value)| (key.into(), value.into()))
                .collect(),
        )
    }
}

impl FromStr for Options {
    type Err = OptionsFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use OptionsFromStrError as E;

        let mismatch = || E::RegexMismatch(s.to_owned());

        let s = s.trim_start().trim_end();

        let mut map = HashMap::new();

        if s.is_empty() {
            return Ok(Self(map));
        }

        let first_re =
            Regex::new(r"^((-c\s+(\w+)=(\w+))|(--(\w+)\s+(\w+)))").map_err(E::InvalidRegex)?;

        let next_re =
            Regex::new(r"\s+((-c\s+(\w+)=(\w+))|(--(\w+)\s+(\w+)))").map_err(E::InvalidRegex)?;

        let mut capture = |re: &Regex, s: &str| {
            let Some(captures) = re.captures(s) else {
                return Err(mismatch());
            };

            let end = captures.get(0).ok_or(E::MissingMatch(0))?.end();

            let (key, value) = match (captures.get(3), captures.get(4)) {
                (Some(key), Some(value)) => (key.as_str(), value.as_str()),
                (None, None) => (
                    captures.get(6).ok_or_else(mismatch)?.as_str(),
                    captures.get(7).ok_or_else(mismatch)?.as_str(),
                ),
                _ => {
                    return Err(mismatch());
                }
            };

            map.insert(key.to_owned().into(), value.to_owned().into());

            Ok::<_, E>(end)
        };

        let mut index = capture(&first_re, s)?;

        while let Some(s) = s.get(index..)
            && !s.is_empty()
        {
            index += capture(&next_re, s)?;
        }

        Ok(Self(map))
    }
}

#[derive(Debug)]
pub enum OptionsFromStrError {
    InvalidRegex(regex::Error),
    RegexMismatch(String),
    MissingMatch(usize),
}

impl Display for Options {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn escape(s: &str) -> String {
            s.replace('\\', r"\\").replace(' ', r"\ ")
        }

        let add = |f: &mut std::fmt::Formatter<'_>, key, value| {
            f.write_str("-c ")?;
            f.write_str(escape(key).as_str())?;
            f.write_char('=')?;
            f.write_str(escape(value).as_str())?;
            Ok::<_, std::fmt::Error>(())
        };

        let mut iter = self.0.iter();

        if let Some((key, value)) = iter.next() {
            add(f, key, value)?;
            for (key, value) in iter {
                f.write_char(' ')?;
                add(f, key, value)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use std::str::FromStr;

    use super::Options;

    #[test]
    fn empty() {
        for s in ["", "  "] {
            assert_eq!(
                s.parse::<Options>().unwrap(),
                Options::new({
                    let arr: [(&'static str, &'static str); 0] = [];
                    arr
                })
            );
        }
    }

    #[test]
    fn many() {
        assert_eq!(
            Options::from_str(" -c key1=value1 -c key2=value2 --key3 value3   ").unwrap(),
            Options::new([("key1", "value1"), ("key2", "value2"), ("key3", "value3")])
        );
    }
}
