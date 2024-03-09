use std::fmt::Debug;

use bb8::RunError;

#[derive(Debug)]
pub enum Error<E, C, Q>
where
    C: Debug,
    Q: Debug,
{
    Pool(RunError<E>),
    Connection(C),
    Query(Q),
}

impl<E, C, Q> From<RunError<E>> for Error<E, C, Q>
where
    C: Debug,
    Q: Debug,
{
    fn from(value: RunError<E>) -> Self {
        Self::Pool(value)
    }
}
