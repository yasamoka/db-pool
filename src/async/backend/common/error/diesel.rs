use diesel::{result::Error, ConnectionError};
use diesel_async::pooled_connection::PoolError;

use crate::r#async::backend::error::Error as BackendError;

impl From<ConnectionError> for BackendError<PoolError, ConnectionError, Error> {
    fn from(value: ConnectionError) -> Self {
        Self::Connection(value)
    }
}

impl From<Error> for BackendError<PoolError, ConnectionError, Error> {
    fn from(value: Error) -> Self {
        Self::Query(value)
    }
}
