use derive_more::{Display, FromStr};

/// SSL TCP/IP connection negotiation priority
#[derive(Debug, Display, Eq, FromStr, PartialEq)]
#[from_str(rename_all = "kebab-case")]
#[display(rename_all = "kebab-case")]
pub enum SslMode {
    /// Only try a non-SSL connection
    Disable,
    /// First try a non-SSL connection; if that fails, try an SSL connection
    Allow,
    /// First try an SSL connection; if that fails, try a non-SSL connection
    Prefer,
    /// Only try an SSL connection. If a root CA file is present, verify the certificate in the same way as if `VerifyCa` was specified
    Require,
    /// Only try an SSL connection, and verify that the server certificate is issued by a trusted certificate authority (CA)
    VerifyCa,
    /// Only try an SSL connection, verify that the server certificate is issued by a trusted CA and that the requested server host name matches that in the certificate
    VerifyFull,
}

#[cfg(test)]
mod tests {
    use super::{super::super::tests::test_serdes, SslMode};

    #[test]
    fn serdes() {
        test_serdes::<SslMode>([
            "disable",
            "allow",
            "prefer",
            "require",
            "verify-ca",
            "verify-full",
        ]);
    }
}
