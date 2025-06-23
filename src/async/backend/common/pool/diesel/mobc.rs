use std::ops::{Deref, DerefMut};

use async_trait::async_trait;
use diesel::{result::Error as DieselError, ConnectionError};
use diesel_async::{
    pooled_connection::{AsyncDieselConnectionManager, PoolError as DieselPoolError},
    AsyncConnection,
};
use mobc::{
    Builder, Connection as MobcConnection, Error as MobcError, Manager as MobcManager, Pool,
};

use crate::r#async::backend::error::Error as BackendError;

use super::r#trait::DieselPoolAssociation;

type DieselManager<Connection> = AsyncDieselConnectionManager<Connection>;

/// [`Diesel mobc`](https://docs.rs/diesel-async/0.5.0/diesel_async/pooled_connection/mobc/index.html) association
/// # Example
/// ```
/// use db_pool::{
///     r#async::{DieselAsyncPostgresBackend, DieselMobc},
///     PrivilegedPostgresConfig,
/// };
/// use diesel::sql_query;
/// use diesel_async::RunQueryDsl;
/// use dotenvy::dotenv;
/// use mobc::Pool;
///
/// async fn f() {
///     dotenv().ok();
///
///     let config = PrivilegedPostgresConfig::from_env().unwrap();
///
///     let backend = DieselAsyncPostgresBackend::<DieselMobc>::new(
///         config,
///         |_| Pool::builder().max_open(10),
///         |_| Pool::builder().max_open(2),
///         None,
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
pub struct DieselMobc;

#[async_trait]
impl<Connection> DieselPoolAssociation<Connection> for DieselMobc
where
    Connection: AsyncConnection + 'static,
    DieselManager<Connection>: MobcManager,
    for<'pool> MobcConnection<DieselManager<Connection>>: DerefMut<Target = Connection>,
    MobcError<<DieselManager<Connection> as MobcManager>::Error>: Into<MobcError<DieselPoolError>>,
{
    type PooledConnection<'pool> = MobcConnection<DieselManager<Connection>>;

    type Builder = Builder<DieselManager<Connection>>;
    type Pool = Pool<DieselManager<Connection>>;

    type BuildError = BuildError;
    type PoolError = PoolError;

    async fn build_pool(
        builder: Builder<DieselManager<Connection>>,
        manager: DieselManager<Connection>,
    ) -> Result<Self::Pool, Self::BuildError> {
        Ok(builder.build(manager))
    }

    async fn get_connection<'pool>(
        pool: &'pool Self::Pool,
    ) -> Result<Self::PooledConnection<'pool>, Self::PoolError> {
        pool.get().await.map_err(|err| err.into().into())
    }
}

#[derive(Debug)]
pub struct BuildError(MobcError<DieselPoolError>);

impl Deref for BuildError {
    type Target = MobcError<DieselPoolError>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<MobcError<DieselPoolError>> for BuildError {
    fn from(value: MobcError<DieselPoolError>) -> Self {
        Self(value)
    }
}

#[derive(Debug)]
pub struct PoolError(MobcError<DieselPoolError>);

impl Deref for PoolError {
    type Target = MobcError<DieselPoolError>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<MobcError<DieselPoolError>> for PoolError {
    fn from(value: MobcError<DieselPoolError>) -> Self {
        Self(value)
    }
}

impl From<BuildError> for BackendError<BuildError, PoolError, ConnectionError, DieselError> {
    fn from(value: BuildError) -> Self {
        Self::Build(value)
    }
}

impl From<PoolError> for BackendError<BuildError, PoolError, ConnectionError, DieselError> {
    fn from(value: PoolError) -> Self {
        Self::Pool(value)
    }
}
