use derive_more::{Display, FromStr};

/// Replication protocol
#[derive(Debug, Display, Eq, FromStr, PartialEq)]
#[from_str(rename_all = "kebab-case")]
#[display(rename_all = "kebab-case")]
pub enum Replication {
    /// Physical replication mode
    True,
    /// Logical replication mode
    Database,
    /// Regular connection, no replication
    False,
}

#[cfg(test)]
mod tests {
    use super::{super::super::tests::test_serdes, Replication};

    #[test]
    fn serdes() {
        test_serdes::<Replication>(["true", "database", "false"]);
    }
}
