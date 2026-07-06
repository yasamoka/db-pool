use derive_more::{Display, FromStr};

/// Require strictness
#[derive(Debug, Display, Eq, FromStr, Hash, PartialEq)]
#[from_str(rename_all = "kebab-case")]
#[display(rename_all = "kebab-case")]
pub enum RequireStrictness {
    /// Require
    Require,
    /// Prefer
    Prefer,
    /// Disable
    Disable,
}

#[cfg(test)]
mod tests {
    use super::{super::super::tests::test_serdes, RequireStrictness};

    #[test]
    fn serdes() {
        test_serdes::<RequireStrictness>(["require", "prefer", "disable"]);
    }
}
