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

pub struct TokioPostgresDeadpool;

#[async_trait]
impl TokioPostgresPoolAssociation for TokioPostgresDeadpool {
    type PooledConnection<'pool> = PooledConnection;

    type Builder = PoolBuilder<Manager>;
    type Pool = Pool<Manager>;

    type BuildError = BuildError<Error>;
    type PoolError = PoolError<Error>;

    async fn build_pool(
        builder: PoolBuilder<Manager>,
        // TODO: add builder wrapper
        _config: Config,
    ) -> Result<Pool<Manager>, BuildError<Error>> {
        builder.build().map_err(Into::into)
    }

    async fn get_connection<'pool>(
        pool: &'pool Pool<Manager>,
    ) -> Result<PooledConnection, PoolError<Error>> {
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

impl From<BuildError<Error>>
    for BackendError<BuildError<Error>, PoolError<Error>, ConnectionError, QueryError>
{
    fn from(value: BuildError<Error>) -> Self {
        Self::Build(value)
    }
}

impl From<PoolError<Error>>
    for BackendError<BuildError<Error>, PoolError<Error>, ConnectionError, QueryError>
{
    fn from(value: PoolError<Error>) -> Self {
        Self::Pool(value)
    }
}
