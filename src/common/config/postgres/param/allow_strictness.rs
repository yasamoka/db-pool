use derive_more::{Display, FromStr};

#[derive(Debug, Display, Eq, FromStr, PartialEq)]
#[from_str(rename_all = "kebab-case")]
#[display(rename_all = "kebab-case")]
pub enum AllowStrictness {
    Require,
    Allow,
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
