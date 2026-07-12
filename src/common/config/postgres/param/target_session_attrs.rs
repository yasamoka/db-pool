use derive_more::{Display, FromStr};

/// Target session attributes
#[derive(Debug, Display, Eq, FromStr, PartialEq)]
#[from_str(rename_all = "kebab-case")]
#[display(rename_all = "kebab-case")]
pub enum TargetSessionAttrs {
    /// Any successful connection is acceptable
    Any,
    /// Session must accept read-write transactions by default (that is, the server must not be in hot standby mode and the `default_transaction_read_only` parameter must be off)
    ReadWrite,
    /// Session must not accept read-write transactions by default (the converse)
    ReadOnly,
    /// Server must not be in hot standby mode
    Primary,
    /// Server must be in hot standby mode
    Standby,
    /// First try to find a standby server, but if none of the listed hosts is a standby server, try again in any mode
    PreferStandby,
}

#[cfg(test)]
mod tests {
    use super::{super::super::tests::test_serdes, TargetSessionAttrs};

    #[test]
    fn serdes() {
        test_serdes::<TargetSessionAttrs>([
            "any",
            "read-write",
            "read-only",
            "primary",
            "standby",
            "prefer-standby",
        ]);
    }
}
