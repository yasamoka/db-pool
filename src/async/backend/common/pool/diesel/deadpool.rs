use async_trait::async_trait;
use deadpool::managed::{BuildError, Object, Pool, PoolBuilder, PoolError as DeadpoolPoolError};
use diesel::{result::Error as DieselError, ConnectionError};
use diesel_async::{
    pooled_connection::{AsyncDieselConnectionManager, PoolError},
    AsyncPgConnection,
};

use crate::r#async::backend::error::Error as BackendError;

use super::r#trait::DieselPoolAssociation;

type DieselManager<Connection> = AsyncDieselConnectionManager<Connection>;

/// [`Diesel deadpool`](https://docs.rs/diesel-async/0.5.0/diesel_async/pooled_connection/deadpool/index.html) association
pub struct DieselDeadpool;

#[async_trait]
impl DieselPoolAssociation<AsyncPgConnection> for DieselDeadpool {
    type PooledConnection<'pool> = Object<DieselManager<AsyncPgConnection>>;

    type Builder = PoolBuilder<DieselManager<AsyncPgConnection>>;
    type Pool = Pool<DieselManager<AsyncPgConnection>>;

    type BuildError = BuildError;
    type PoolError = DeadpoolPoolError<PoolError>;

    async fn build_pool(
        builder: Self::Builder,
        _: DieselManager<AsyncPgConnection>,
    ) -> Result<Self::Pool, Self::BuildError> {
        builder.build().map_err(Into::into)
    }

    async fn get_connection<'pool>(
        pool: &'pool Self::Pool,
    ) -> Result<Self::PooledConnection<'pool>, Self::PoolError> {
        pool.get().await.map_err(Into::into)
    }
}

impl From<BuildError>
    for BackendError<BuildError, DeadpoolPoolError<PoolError>, ConnectionError, DieselError>
{
    fn from(value: BuildError) -> Self {
        Self::Build(value)
    }
}

impl From<DeadpoolPoolError<PoolError>>
    for BackendError<BuildError, DeadpoolPoolError<PoolError>, ConnectionError, DieselError>
{
    fn from(value: DeadpoolPoolError<PoolError>) -> Self {
        Self::Pool(value)
    }
}
