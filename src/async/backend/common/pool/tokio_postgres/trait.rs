use std::{fmt::Debug, ops::DerefMut};

use async_trait::async_trait;
use tokio_postgres::{Client, Config};

use crate::r#async::backend::{
    common::error::tokio_postgres::{ConnectionError, QueryError},
    error::Error as BackendError,
};

#[async_trait]
pub trait TokioPostgresPoolAssociation: 'static {
    type PooledConnection<'pool>: DerefMut<Target = Client> + Send;

    type Builder;
    type Pool: Send + Sync + 'static;

    type BuildError: Into<BackendError<Self::BuildError, Self::PoolError, ConnectionError, QueryError>>
        + Debug
        + Send;
    type PoolError: Into<BackendError<Self::BuildError, Self::PoolError, ConnectionError, QueryError>>
        + Debug
        + Send;

    async fn build_pool(
        builder: Self::Builder,
        config: Config,
    ) -> Result<Self::Pool, Self::BuildError>;
    async fn get_connection<'pool>(
        pool: &'pool Self::Pool,
    ) -> Result<Self::PooledConnection<'pool>, Self::PoolError>;
}
