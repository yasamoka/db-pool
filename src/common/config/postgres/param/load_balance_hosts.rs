use derive_more::{Display, FromStr};

/// Host load balancing configuration
#[derive(Debug, Display, Eq, FromStr, PartialEq)]
#[from_str(rename_all = "kebab-case")]
#[display(rename_all = "kebab-case")]
pub enum LoadBalanceHosts {
    /// No load balancing across hosts
    Disable,
    /// Hosts and addresses are tried in random order
    Random,
}

#[cfg(test)]
mod tests {
    use super::{super::super::tests::test_serdes, LoadBalanceHosts};

    #[test]
    fn serdes() {
        test_serdes::<LoadBalanceHosts>(["disable", "random"]);
    }
}
