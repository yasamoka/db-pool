use derive_more::{Display, FromStr};

/// GSS library
#[derive(Debug, Display, Eq, FromStr, PartialEq)]
#[from_str(rename_all = "kebab-case")]
#[display(rename_all = "kebab-case")]
pub enum GssLib {
    /// gssapi
    Gssapi,
}

#[cfg(test)]
mod tests {
    use super::{super::super::tests::test_serdes, GssLib};

    #[test]
    fn serdes() {
        test_serdes::<GssLib>(["gssapi"]);
    }
}
