use std::ops::Deref;

use sea_orm::DbErr;

use crate::r#async::backend::error::Error as BackendError;

#[derive(Debug)]
pub struct BuildError(DbErr);

impl Deref for BuildError {
    type Target = DbErr;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<DbErr> for BuildError {
    fn from(value: DbErr) -> Self {
        Self(value)
    }
}

#[derive(Debug)]
pub struct PoolError(DbErr);

impl Deref for PoolError {
    type Target = DbErr;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<DbErr> for PoolError {
    fn from(value: DbErr) -> Self {
        Self(value)
    }
}

#[derive(Debug)]
pub struct ConnectionError(DbErr);

impl Deref for ConnectionError {
    type Target = DbErr;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<DbErr> for ConnectionError {
    fn from(value: DbErr) -> Self {
        Self(value)
    }
}

#[derive(Debug)]
pub struct QueryError(DbErr);

impl Deref for QueryError {
    type Target = DbErr;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<DbErr> for QueryError {
    fn from(value: DbErr) -> Self {
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
