use std::fmt::Debug;

use r2d2::{ManageConnection, Pool};
use uuid::Uuid;

use super::error::Error;

pub trait Backend: Sized + Send + Sync + 'static {
    type ConnectionManager: ManageConnection;
    type ConnectionError: Debug;
    type QueryError: Debug;

    fn init(&self) -> Result<(), Error<Self::ConnectionError, Self::QueryError>>;
    #[allow(clippy::complexity)]
    fn create(
        &self,
        db_id: Uuid,
    ) -> Result<Pool<Self::ConnectionManager>, Error<Self::ConnectionError, Self::QueryError>>;
    fn clean(&self, db_id: Uuid) -> Result<(), Error<Self::ConnectionError, Self::QueryError>>;
    fn drop(&self, db_id: Uuid) -> Result<(), Error<Self::ConnectionError, Self::QueryError>>;
}
