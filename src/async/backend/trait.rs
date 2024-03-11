use std::fmt::Debug;

use async_trait::async_trait;
use uuid::Uuid;

use super::error::Error;

#[async_trait]
pub trait Backend: Sized + Send + Sync + 'static {
    type Pool: Send;

    type BuildError: Debug + Send;
    type PoolError: Debug + Send;
    type ConnectionError: Debug;
    type QueryError: Debug;

    async fn init(
        &self,
    ) -> Result<(), Error<Self::BuildError, Self::PoolError, Self::ConnectionError, Self::QueryError>>;
    #[allow(clippy::complexity)]
    async fn create(
        &self,
        db_id: Uuid,
    ) -> Result<
        Self::Pool,
        Error<Self::BuildError, Self::PoolError, Self::ConnectionError, Self::QueryError>,
    >;
    async fn clean(
        &self,
        db_id: Uuid,
    ) -> Result<(), Error<Self::BuildError, Self::PoolError, Self::ConnectionError, Self::QueryError>>;
    async fn drop(
        &self,
        db_id: Uuid,
    ) -> Result<(), Error<Self::BuildError, Self::PoolError, Self::ConnectionError, Self::QueryError>>;
}
