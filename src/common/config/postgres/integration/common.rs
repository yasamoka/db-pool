use super::super::param::{
    LoadBalanceHosts, RequireStrictness, SslMode, SslNegotiation, TargetSessionAttrs,
};

impl From<RequireStrictness> for tokio_postgres::config::ChannelBinding {
    fn from(value: RequireStrictness) -> Self {
        use RequireStrictness as T;
        use tokio_postgres::config::ChannelBinding as U;
        match value {
            T::Disable => U::Disable,
            T::Require => U::Require,
            T::Prefer => U::Prefer,
        }
    }
}

impl From<SslMode> for Option<tokio_postgres::config::SslMode> {
    fn from(value: SslMode) -> Self {
        use SslMode as T;
        use tokio_postgres::config::SslMode as U;
        match value {
            T::Disable => Some(U::Disable),
            T::Prefer => Some(U::Prefer),
            T::Require => Some(U::Require),
            T::Allow | T::VerifyCa | T::VerifyFull => None,
        }
    }
}

impl From<SslNegotiation> for tokio_postgres::config::SslNegotiation {
    fn from(value: SslNegotiation) -> Self {
        use SslNegotiation as T;
        use tokio_postgres::config::SslNegotiation as U;
        match value {
            T::Direct => U::Direct,
            T::Postgres => U::Postgres,
        }
    }
}

impl From<TargetSessionAttrs> for Option<tokio_postgres::config::TargetSessionAttrs> {
    fn from(value: TargetSessionAttrs) -> Self {
        use TargetSessionAttrs as T;
        use tokio_postgres::config::TargetSessionAttrs as U;
        match value {
            T::Any => Some(U::Any),
            T::ReadOnly => Some(U::ReadOnly),
            T::ReadWrite => Some(U::ReadWrite),
            T::PreferStandby | T::Primary | T::Standby => None,
        }
    }
}

impl From<LoadBalanceHosts> for tokio_postgres::config::LoadBalanceHosts {
    fn from(value: LoadBalanceHosts) -> Self {
        use LoadBalanceHosts as T;
        use tokio_postgres::config::LoadBalanceHosts as U;
        match value {
            T::Disable => U::Disable,
            T::Random => U::Random,
        }
    }
}
