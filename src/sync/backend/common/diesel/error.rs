use diesel::{result::Error, ConnectionError};

use super::super::super::error::Error as BackendError;

impl From<ConnectionError> for BackendError<ConnectionError, Error> {
    fn from(value: ConnectionError) -> Self {
        Self::Connection(value)
    }
}

impl From<Error> for BackendError<ConnectionError, Error> {
    fn from(value: Error) -> Self {
        Self::Query(value)
    }
}
