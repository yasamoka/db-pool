use derive_more::{Display, FromStr};

/// Allow strictness
#[derive(Debug, Display, Eq, FromStr, PartialEq)]
#[from_str(rename_all = "kebab-case")]
#[display(rename_all = "kebab-case")]
pub enum AllowStrictness {
    /// Require
    Require,
    /// Allow
    Allow,
    /// Disable
    Disable,
}

#[cfg(test)]
mod tests {
    use super::{super::super::tests::test_serdes, AllowStrictness};

    #[test]
    fn serdes() {
        test_serdes::<AllowStrictness>(["require", "allow", "disable"]);
    }
}
