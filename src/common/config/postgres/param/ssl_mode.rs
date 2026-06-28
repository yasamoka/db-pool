use derive_more::{Display, FromStr};

#[derive(Debug, Display, Eq, FromStr, PartialEq)]
#[from_str(rename_all = "kebab-case")]
#[display(rename_all = "kebab-case")]
pub enum SslMode {
    Disable,
    Allow,
    Prefer,
    Require,
    VerifyCa,
    VerifyFull,
}

#[cfg(test)]
mod tests {
    use super::{super::super::tests::test_serdes, SslMode};

    #[test]
    fn serdes() {
        test_serdes::<SslMode>([
            "disable",
            "allow",
            "prefer",
            "require",
            "verify-ca",
            "verify-full",
        ]);
    }
}
