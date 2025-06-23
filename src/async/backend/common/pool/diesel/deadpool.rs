use std::ops::DerefMut;

use async_trait::async_trait;
use deadpool::managed::{
    BuildError, Manager as DeadpoolManager, Object, Pool, PoolBuilder,
    PoolError as DeadpoolPoolError,
};
use diesel::{ConnectionError, result::Error as DieselError};
use diesel_async::{
    AsyncConnection,
    pooled_connection::{AsyncDieselConnectionManager, PoolError as DieselPoolError},
};

use crate::r#async::backend::error::Error as BackendError;

use super::r#trait::DieselPoolAssociation;

type DieselManager<Connection> = AsyncDieselConnectionManager<Connection>;

/// [`Diesel deadpool`](https://docs.rs/diesel-async/0.5.2/diesel_async/pooled_connection/deadpool/index.html) association
pub struct DieselDeadpool;

#[async_trait]
impl<Connection> DieselPoolAssociation<Connection> for DieselDeadpool
where
    Connection: AsyncConnection + 'static,
    DieselManager<Connection>: DeadpoolManager,
    for<'pool> Object<DieselManager<Connection>>: DerefMut<Target = Connection>,
    DeadpoolPoolError<<DieselManager<Connection> as DeadpoolManager>::Error>:
        Into<DeadpoolPoolError<DieselPoolError>>,
{
    type PooledConnection<'pool> = Object<DieselManager<Connection>>;

    type Builder = PoolBuilder<DieselManager<Connection>>;
    type Pool = Pool<DieselManager<Connection>>;

    type BuildError = BuildError;
    type PoolError = DeadpoolPoolError<DieselPoolError>;

    async fn build_pool(
        builder: Self::Builder,
        _: DieselManager<Connection>,
    ) -> Result<Self::Pool, Self::BuildError> {
        builder.build()
    }

    async fn get_connection<'pool>(
        pool: &'pool Self::Pool,
    ) -> Result<Self::PooledConnection<'pool>, Self::PoolError> {
        pool.get().await.map_err(Into::into)
    }
}

impl From<BuildError>
    for BackendError<BuildError, DeadpoolPoolError<DieselPoolError>, ConnectionError, DieselError>
{
    fn from(value: BuildError) -> Self {
        Self::Build(value)
    }
}

impl From<DeadpoolPoolError<DieselPoolError>>
    for BackendError<BuildError, DeadpoolPoolError<DieselPoolError>, ConnectionError, DieselError>
{
    fn from(value: DeadpoolPoolError<DieselPoolError>) -> Self {
        Self::Pool(value)
    }
}
