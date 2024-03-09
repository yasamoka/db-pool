use std::fmt::Debug;

#[derive(Debug)]
pub enum Error<C, Q>
where
    C: Debug,
    Q: Debug,
{
    Pool(r2d2::Error),
    Connection(C),
    Query(Q),
}

impl<C, Q> From<r2d2::Error> for Error<C, Q>
where
    C: Debug,
    Q: Debug,
{
    fn from(value: r2d2::Error) -> Self {
        Self::Pool(value)
    }
}
