use std::fmt::Debug;

#[derive(Debug)]
pub enum Error<B: Debug, P: Debug, C: Debug, Q: Debug> {
    Build(B),
    Pool(P),
    Connection(C),
    Query(Q),
}
