use std::fmt::Debug;

use async_trait::async_trait;
use bb8::{ManageConnection, Pool};
use uuid::Uuid;

use super::error::Error;

#[async_trait]
pub trait Backend: Sized + Send + Sync + 'static {
    type ConnectionManager: ManageConnection;
    type ConnectionError: Debug;
    type QueryError: Debug;

    async fn init(
        &self,
    ) -> Result<
        (),
        Error<
            <Self::ConnectionManager as ManageConnection>::Error,
            Self::ConnectionError,
            Self::QueryError,
        >,
    >;
    #[allow(clippy::complexity)]
    async fn create(
        &self,
        db_id: Uuid,
    ) -> Result<
        Pool<Self::ConnectionManager>,
        Error<
            <Self::ConnectionManager as ManageConnection>::Error,
            Self::ConnectionError,
            Self::QueryError,
        >,
    >;
    async fn clean(
        &self,
        db_id: Uuid,
    ) -> Result<
        (),
        Error<
            <Self::ConnectionManager as ManageConnection>::Error,
            Self::ConnectionError,
            Self::QueryError,
        >,
    >;
    async fn drop(
        &self,
        db_id: Uuid,
    ) -> Result<
        (),
        Error<
            <Self::ConnectionManager as ManageConnection>::Error,
            Self::ConnectionError,
            Self::QueryError,
        >,
    >;
}
