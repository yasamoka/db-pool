use std::{fmt::Debug, ops::DerefMut};

use async_trait::async_trait;
use diesel::{result::Error, ConnectionError};
use diesel_async::{pooled_connection::AsyncDieselConnectionManager, AsyncConnection};

use crate::r#async::backend::error::Error as BackendError;

#[async_trait]
pub trait DieselPoolAssociation<Connection>: 'static
where
    Connection: AsyncConnection + 'static,
{
    type PooledConnection<'pool>: DerefMut<Target = Connection> + Send;

    type Builder;
    type Pool: Send + Sync + 'static;

    type BuildError: Into<BackendError<Self::BuildError, Self::PoolError, ConnectionError, Error>>
        + Debug
        + Send;
    type PoolError: Into<BackendError<Self::BuildError, Self::PoolError, ConnectionError, Error>>
        + Debug
        + Send;

    async fn build_pool(
        builder: Self::Builder,
        manager: AsyncDieselConnectionManager<Connection>,
    ) -> Result<Self::Pool, Self::BuildError>;
    async fn get_connection<'pool>(
        pool: &'pool Self::Pool,
    ) -> Result<Self::PooledConnection<'pool>, Self::PoolError>;
}
