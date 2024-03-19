use std::fmt::Debug;

use async_trait::async_trait;
use uuid::Uuid;

use super::error::Error;

/// Backend trait
#[async_trait]
pub trait Backend: Sized + Send + Sync + 'static {
    /// Connection pool type that implements ``Send``
    type Pool: Send;

    /// Connection pool build error type that implements ``Debug`` and ``Send``
    type BuildError: Debug + Send;
    /// Connection pool error type that implements ``Debug`` and ``Send``
    type PoolError: Debug + Send;
    /// Connection error type that implements ``Debug``
    type ConnectionError: Debug;
    /// Query error type that implements ``Debug``
    type QueryError: Debug;

    /// Initializes the backend
    async fn init(
        &self,
    ) -> Result<(), Error<Self::BuildError, Self::PoolError, Self::ConnectionError, Self::QueryError>>;
    #[allow(clippy::complexity)]

    /// Creates a database
    async fn create(
        &self,
        db_id: Uuid,
    ) -> Result<
        Self::Pool,
        Error<Self::BuildError, Self::PoolError, Self::ConnectionError, Self::QueryError>,
    >;

    /// Cleans a database
    async fn clean(
        &self,
        db_id: Uuid,
    ) -> Result<(), Error<Self::BuildError, Self::PoolError, Self::ConnectionError, Self::QueryError>>;

    /// Drops a database
    async fn drop(
        &self,
        db_id: Uuid,
    ) -> Result<(), Error<Self::BuildError, Self::PoolError, Self::ConnectionError, Self::QueryError>>;
}
