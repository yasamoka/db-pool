use std::fmt::Debug;

#[derive(Debug)]
pub enum Error<C: Debug, Q: Debug> {
    Pool(r2d2::Error),
    Connection(C),
    Query(Q),
}

impl<C: Debug, Q: Debug> From<r2d2::Error> for Error<C, Q> {
    fn from(value: r2d2::Error) -> Self {
        Self::Pool(value)
    }
}
