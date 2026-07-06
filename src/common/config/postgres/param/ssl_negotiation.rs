use derive_more::{Display, FromStr};

/// SSL encryption negotiation
#[derive(Debug, Display, Eq, FromStr, PartialEq)]
#[from_str(rename_all = "kebab-case")]
#[display(rename_all = "kebab-case")]
pub enum SslNegotiation {
    /// Client first asks server if SSL is supported
    Postgres,
    /// Client starts standard SSL handshake directly after establishing TCP/IP connection
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
