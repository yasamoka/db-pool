use std::ops::{Deref, DerefMut};

use async_trait::async_trait;
use bb8::{Builder, ManageConnection, Pool, PooledConnection, RunError};
use diesel::{result::Error, ConnectionError};
use diesel_async::{
    pooled_connection::{AsyncDieselConnectionManager as Manager, PoolError as DieselPoolError},
    AsyncConnection,
};

use crate::r#async::backend::error::Error as BackendError;

use super::r#trait::DieselPoolAssociation;

/// ``Diesel`` ``bb8`` association
/// # Example
/// ```
/// use bb8::Pool;
/// use db_pool::{
///     r#async::{DieselAsyncPgBackend, DieselBb8},
///     PrivilegedPostgresConfig,
/// };
/// use diesel::sql_query;
/// use diesel_async::RunQueryDsl;
///
/// async fn f() {
///     let backend = DieselAsyncPgBackend::<DieselBb8>::new(
///         PrivilegedPostgresConfig::new("postgres".to_owned())
///             .password(Some("postgres".to_owned())),
///         || Pool::builder().max_size(10),
///         || Pool::builder().max_size(2),
///         move |mut conn| {
///             Box::pin(async {
///                 sql_query("CREATE TABLE book(id SERIAL PRIMARY KEY, title TEXT NOT NULL)")
///                     .execute(&mut conn)
///                     .await
///                     .unwrap();
///                 conn
///             })
///         },
///     )
///     .await
///     .unwrap();
/// }
///
/// tokio_test::block_on(f());
/// ```
pub struct DieselBb8;

#[async_trait]
impl<Connection> DieselPoolAssociation<Connection> for DieselBb8
where
    Connection: AsyncConnection + 'static,
    Manager<Connection>: ManageConnection,
    for<'pool> PooledConnection<'pool, Manager<Connection>>: DerefMut<Target = Connection>,
    <Manager<Connection> as ManageConnection>::Error: Into<RunError<DieselPoolError>>,
    RunError<<Manager<Connection> as ManageConnection>::Error>: Into<RunError<DieselPoolError>>,
{
    type PooledConnection<'pool> = PooledConnection<'pool, Manager<Connection>>;

    type Builder = Builder<Manager<Connection>>;
    type Pool = Pool<Manager<Connection>>;

    type BuildError = BuildError;
    type PoolError = PoolError;

    async fn build_pool(
        builder: Self::Builder,
        manager: Manager<Connection>,
    ) -> Result<Self::Pool, Self::BuildError> {
        builder
            .build(manager)
            .await
            .map_err(|err| err.into().into())
    }

    async fn get_connection<'pool>(
        pool: &'pool Self::Pool,
    ) -> Result<Self::PooledConnection<'pool>, Self::PoolError> {
        pool.get().await.map_err(|err| err.into().into())
    }
}

#[derive(Debug)]
pub struct BuildError(RunError<DieselPoolError>);

impl Deref for BuildError {
    type Target = RunError<DieselPoolError>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<RunError<DieselPoolError>> for BuildError {
    fn from(value: RunError<DieselPoolError>) -> Self {
        Self(value)
    }
}

#[derive(Debug)]
pub struct PoolError(RunError<DieselPoolError>);

impl Deref for PoolError {
    type Target = RunError<DieselPoolError>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<RunError<DieselPoolError>> for PoolError {
    fn from(value: RunError<DieselPoolError>) -> Self {
        Self(value)
    }
}

impl From<BuildError> for BackendError<BuildError, PoolError, ConnectionError, Error> {
    fn from(value: BuildError) -> Self {
        Self::Build(value)
    }
}

impl From<PoolError> for BackendError<BuildError, PoolError, ConnectionError, Error> {
    fn from(value: PoolError) -> Self {
        Self::Pool(value)
    }
}
