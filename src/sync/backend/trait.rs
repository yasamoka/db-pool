use std::fmt::Debug;

use r2d2::{ManageConnection, Pool};
use uuid::Uuid;

use super::error::Error;

/// Backend trait
pub trait Backend: Sized + Send + Sync + 'static {
    /// Type that implements the [`r2d2::ManageConnection`](https://docs.rs/r2d2/0.8.10/r2d2/trait.ManageConnection.html) trait
    type ConnectionManager: ManageConnection;
    /// Connection error type that implements [`Debug`](https://doc.rust-lang.org/std/fmt/trait.Debug.html)
    type ConnectionError: Debug;
    /// Query error type that implements [`Debug`](https://doc.rust-lang.org/std/fmt/trait.Debug.html)
    type QueryError: Debug;

    /// Initializes the backend
    fn init(&self) -> Result<(), Error<Self::ConnectionError, Self::QueryError>>;

    /// Creates a database
    #[allow(clippy::complexity)]
    fn create(
        &self,
        db_id: Uuid,
        restrict_privileges: bool,
    ) -> Result<Pool<Self::ConnectionManager>, Error<Self::ConnectionError, Self::QueryError>>;

    /// Cleans a database
    fn clean(&self, db_id: Uuid) -> Result<(), Error<Self::ConnectionError, Self::QueryError>>;

    /// Drops a database
    fn drop(
        &self,
        db_id: Uuid,
        is_restricted: bool,
    ) -> Result<(), Error<Self::ConnectionError, Self::QueryError>>;
}
