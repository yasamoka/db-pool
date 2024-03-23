use std::fmt::Debug;

use diesel::{result::Error, ConnectionError};

use crate::r#async::backend::error::Error as BackendError;

impl<B: Debug, P: Debug> From<ConnectionError> for BackendError<B, P, ConnectionError, Error> {
    fn from(value: ConnectionError) -> Self {
        Self::Connection(value)
    }
}

impl<B: Debug, P: Debug> From<Error> for BackendError<B, P, ConnectionError, Error> {
    fn from(value: Error) -> Self {
        Self::Query(value)
    }
}
