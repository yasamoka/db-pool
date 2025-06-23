use std::ops::{Deref, DerefMut};

use async_trait::async_trait;
use deadpool::managed::{BuildError, Object, Pool, PoolBuilder, PoolError};
use deadpool_postgres::Manager;
use tokio_postgres::{Client, Config, Error};

use crate::r#async::backend::{
    common::error::tokio_postgres::{ConnectionError, QueryError},
    error::Error as BackendError,
};

use super::r#trait::TokioPostgresPoolAssociation;

/// [`tokio-postgres deadpool`](https://docs.rs/deadpool-postgres/0.14.1/deadpool_postgres/) association
pub struct TokioPostgresDeadpool;

#[async_trait]
impl TokioPostgresPoolAssociation for TokioPostgresDeadpool {
    type PooledConnection<'pool> = PooledConnection;

    type Builder = PoolBuilder<Manager>;
    type Pool = Pool<Manager>;

    type BuildError = BuildError;
    type PoolError = PoolError<Error>;

    async fn build_pool(
        builder: PoolBuilder<Manager>,
        _config: Config,
    ) -> Result<Self::Pool, Self::BuildError> {
        builder.build()
    }

    async fn get_connection<'pool>(
        pool: &'pool Pool<Manager>,
    ) -> Result<Self::PooledConnection<'pool>, Self::PoolError> {
        pool.get().await.map(Into::into)
    }
}

pub struct PooledConnection(Object<Manager>);

impl From<Object<Manager>> for PooledConnection {
    fn from(value: Object<Manager>) -> Self {
        Self(value)
    }
}

impl Deref for PooledConnection {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for PooledConnection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<BuildError> for BackendError<BuildError, PoolError<Error>, ConnectionError, QueryError> {
    fn from(value: BuildError) -> Self {
        Self::Build(value)
    }
}

impl From<PoolError<Error>>
    for BackendError<BuildError, PoolError<Error>, ConnectionError, QueryError>
{
    fn from(value: PoolError<Error>) -> Self {
        Self::Pool(value)
    }
}
