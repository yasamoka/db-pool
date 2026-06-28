mod config;
mod host;
mod integration;
mod param;
mod params;

pub use config::PrivilegedPostgresConfig;

#[cfg(test)]
pub(super) mod tests {
    #![allow(clippy::unwrap_used)]

    use std::{fmt::Debug, str::FromStr};

    pub(super) fn test_serdes<T>(options: impl IntoIterator<Item = &'static str>)
    where
        T: FromStr + ToString,
        <T as FromStr>::Err: Debug,
    {
        for option in options {
            let e = option.parse::<T>().unwrap();
            let s = e.to_string();
            assert_eq!(s, option);
        }
    }
}
