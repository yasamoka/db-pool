use std::fmt::Debug;

use tokio_postgres::Error;

use crate::r#async::backend::error::Error as BackendError;

#[derive(Debug)]
pub struct ConnectionError(Error);

impl From<Error> for ConnectionError {
    fn from(value: Error) -> Self {
        Self(value)
    }
}

#[derive(Debug)]
pub struct QueryError(Error);

impl From<Error> for QueryError {
    fn from(value: Error) -> Self {
        Self(value)
    }
}

impl<B, P> From<ConnectionError> for BackendError<B, P, ConnectionError, QueryError>
where
    B: Debug,
    P: Debug,
{
    fn from(value: ConnectionError) -> Self {
        Self::Connection(value)
    }
}

impl<B, P> From<QueryError> for BackendError<B, P, ConnectionError, QueryError>
where
    B: Debug,
    P: Debug,
{
    fn from(value: QueryError) -> Self {
        Self::Query(value)
    }
}
