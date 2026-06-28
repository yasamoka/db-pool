use std::str::FromStr;

use derive_more::Display;

#[derive(Debug, Display, Eq, PartialEq)]
pub enum ProtocolVersion {
    #[display("3.0")]
    V3_0,
    #[display("3.2")]
    V3_2,
    #[display("latest")]
    Latest,
}

impl FromStr for ProtocolVersion {
    type Err = ProtocolVersionFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "3.0" => Ok(Self::V3_0),
            "3.2" => Ok(Self::V3_2),
            "latest" => Ok(Self::Latest),
            s => Err(ProtocolVersionFromStrError(s.to_owned())),
        }
    }
}

#[derive(Debug)]
pub struct ProtocolVersionFromStrError(pub String);

#[cfg(test)]
mod tests {
    use super::{super::super::tests::test_serdes, ProtocolVersion};

    #[test]
    fn serdes() {
        test_serdes::<ProtocolVersion>(["3.0", "3.2", "latest"]);
    }
}
