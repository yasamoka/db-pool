use derive_more::{Display, FromStr};

#[derive(Debug, Display, Eq, FromStr, PartialEq)]
#[from_str(rename_all = "kebab-case")]
#[display(rename_all = "kebab-case")]
pub enum TargetSessionAttrs {
    Any,
    ReadWrite,
    ReadOnly,
    Primary,
    Standby,
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
