use std::ops::Deref;

use async_trait::async_trait;
use bb8::{Builder, Pool, PooledConnection, RunError};
use bb8_postgres::PostgresConnectionManager;
use tokio_postgres::{Config, Error, NoTls};

use crate::r#async::backend::{
    common::error::tokio_postgres::{ConnectionError, QueryError},
    error::Error as BackendError,
};

use super::r#trait::TokioPostgresPoolAssociation;

type Manager = PostgresConnectionManager<NoTls>;

pub struct TokioPostgresBb8;

#[async_trait]
impl TokioPostgresPoolAssociation for TokioPostgresBb8 {
    type PooledConnection<'pool> = PooledConnection<'pool, Manager>;

    type Builder = Builder<Manager>;
    type Pool = Pool<Manager>;

    type BuildError = BuildError;
    type PoolError = PoolError;

    async fn build_pool(
        builder: Builder<Manager>,
        config: Config,
    ) -> Result<Pool<Manager>, BuildError> {
        let manager = Manager::new(config, NoTls);
        builder.build(manager).await.map_err(Into::into)
    }

    async fn get_connection<'pool>(
        pool: &'pool Self::Pool,
    ) -> Result<Self::PooledConnection<'pool>, Self::PoolError> {
        pool.get().await.map_err(Into::into)
    }
}

#[derive(Debug)]
pub struct BuildError(Error);

impl Deref for BuildError {
    type Target = Error;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Error> for BuildError {
    fn from(value: Error) -> Self {
        Self(value)
    }
}

#[derive(Debug)]
pub struct PoolError(RunError<Error>);

impl Deref for PoolError {
    type Target = RunError<Error>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<RunError<Error>> for PoolError {
    fn from(value: RunError<Error>) -> Self {
        Self(value)
    }
}

impl From<BuildError> for BackendError<BuildError, PoolError, ConnectionError, QueryError> {
    fn from(value: BuildError) -> Self {
        Self::Build(value)
    }
}

impl From<PoolError> for BackendError<BuildError, PoolError, ConnectionError, QueryError> {
    fn from(value: PoolError) -> Self {
        Self::Pool(value)
    }
}