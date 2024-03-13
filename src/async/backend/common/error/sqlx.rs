use std::ops::Deref;

use sqlx::Error;

use crate::r#async::backend::error::Error as BackendError;

#[derive(Debug)]
pub struct BuildError;

#[derive(Debug)]
pub struct PoolError(Error);

impl Deref for PoolError {
    type Target = Error;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Error> for PoolError {
    fn from(value: Error) -> Self {
        Self(value)
    }
}

#[derive(Debug)]
pub struct ConnectionError(Error);

impl Deref for ConnectionError {
    type Target = Error;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Error> for ConnectionError {
    fn from(value: Error) -> Self {
        Self(value)
    }
}

#[derive(Debug)]
pub struct QueryError(Error);

impl Deref for QueryError {
    type Target = Error;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Error> for QueryError {
    fn from(value: Error) -> Self {
        Self(value)
    }
}

type BError = BackendError<BuildError, PoolError, ConnectionError, QueryError>;

impl From<BuildError> for BError {
    fn from(value: BuildError) -> Self {
        Self::Build(value)
    }
}

impl From<PoolError> for BError {
    fn from(value: PoolError) -> Self {
        Self::Pool(value)
    }
}

impl From<ConnectionError> for BError {
    fn from(value: ConnectionError) -> Self {
        Self::Connection(value)
    }
}

impl From<QueryError> for BError {
    fn from(value: QueryError) -> Self {
        Self::Query(value)
    }
}
