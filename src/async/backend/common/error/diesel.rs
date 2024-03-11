use std::fmt::Debug;

use diesel::{result::Error, ConnectionError};

use crate::r#async::backend::error::Error as BackendError;

impl<B, P> From<ConnectionError> for BackendError<B, P, ConnectionError, Error>
where
    B: Debug,
    P: Debug,
{
    fn from(value: ConnectionError) -> Self {
        Self::Connection(value)
    }
}

impl<B, P> From<Error> for BackendError<B, P, ConnectionError, Error>
where
    B: Debug,
    P: Debug,
{
    fn from(value: Error) -> Self {
        Self::Query(value)
    }
}
