use derive_more::{Display, FromStr};

#[derive(Debug, Display, Eq, FromStr, PartialEq)]
#[from_str(rename_all = "kebab-case")]
#[display(rename_all = "kebab-case")]
pub enum SslNegotiation {
    Postgres,
    Direct,
}

#[cfg(test)]
mod tests {
    use super::{super::super::tests::test_serdes, SslNegotiation};

    #[test]
    fn serdes() {
        test_serdes::<SslNegotiation>(["postgres", "direct"]);
    }
}
