use std::str::FromStr;

use derive_more::Display;

#[derive(Debug, Display, Eq, PartialEq)]
pub enum SslProtocolVersion {
    #[display("TLSv1")]
    TLSv1,
    #[display("TLSv1.1")]
    TLSv1_1,
    #[display("TLSv1.2")]
    TLSv1_2,
    #[display("TLSv1.3")]
    TLSv1_3,
}

impl FromStr for SslProtocolVersion {
    type Err = SslProtocolVersionFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "TLSv1" => Ok(Self::TLSv1),
            "TLSv1.1" => Ok(Self::TLSv1_1),
            "TLSv1.2" => Ok(Self::TLSv1_2),
            "TLSv1.3" => Ok(Self::TLSv1_3),
            s => Err(SslProtocolVersionFromStrError(s.to_owned())),
        }
    }
}

#[derive(Debug)]
pub struct SslProtocolVersionFromStrError(pub String);

#[cfg(test)]
mod tests {
    use super::{super::super::tests::test_serdes, SslProtocolVersion};

    #[test]
    fn serdes() {
        test_serdes::<SslProtocolVersion>(["TLSv1", "TLSv1.1", "TLSv1.2", "TLSv1.3"]);
    }
}
