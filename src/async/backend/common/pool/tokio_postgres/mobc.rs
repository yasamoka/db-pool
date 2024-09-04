use std::ops::Deref;

use async_trait::async_trait;
use mobc::{Builder, Connection, Error as MobcError, Pool};
use mobc_postgres::PgConnectionManager;
use tokio_postgres::{Config, Error, NoTls};

use crate::r#async::backend::{
    common::error::tokio_postgres::{ConnectionError, QueryError},
    error::Error as BackendError,
};

use super::r#trait::TokioPostgresPoolAssociation;

type Manager = PgConnectionManager<NoTls>;

/// [`tokio-postgres mobc`](https://docs.rs/mobc-postgres/0.8.0/mobc_postgres/) association
/// # Example
/// ```
/// use db_pool::r#async::{TokioPostgresBackend, TokioPostgresMobc};
/// use mobc::Pool;
/// use tokio_postgres::Config;
///
/// async fn f() {
///     let backend = TokioPostgresBackend::<TokioPostgresMobc>::new(
///         "host=localhost user=postgres password=postgres"
///             .parse::<Config>()
///             .unwrap(),
///         || Pool::builder().max_open(10),
///         || Pool::builder().max_open(2),
///         move |conn| {
///             Box::pin(async move {
///                 conn.execute(
///                     "CREATE TABLE book(id SERIAL PRIMARY KEY, title TEXT NOT NULL)",
///                     &[],
///                 )
///                 .await
///                 .unwrap();
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
pub struct TokioPostgresMobc;

#[async_trait]
impl TokioPostgresPoolAssociation for TokioPostgresMobc {
    type PooledConnection<'pool> = Connection<Manager>;

    type Builder = Builder<Manager>;
    type Pool = Pool<Manager>;

    type BuildError = BuildError;
    type PoolError = PoolError;

    async fn build_pool(
        builder: Builder<Manager>,
        config: Config,
    ) -> Result<Self::Pool, Self::BuildError> {
        let manager = Manager::new(config, NoTls);
        Ok(builder.build(manager))
    }

    async fn get_connection<'pool>(
        pool: &'pool Self::Pool,
    ) -> Result<Connection<Manager>, PoolError> {
        pool.get().await.map_err(Into::into)
    }
}

#[derive(Debug)]
pub struct BuildError(MobcError<Error>);

impl Deref for BuildError {
    type Target = MobcError<Error>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<MobcError<Error>> for BuildError {
    fn from(value: MobcError<Error>) -> Self {
        Self(value)
    }
}

#[derive(Debug)]
pub struct PoolError(MobcError<Error>);

impl Deref for PoolError {
    type Target = MobcError<Error>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<MobcError<Error>> for PoolError {
    fn from(value: MobcError<Error>) -> Self {
        Self(value)
    }
}

impl From<BuildError> for BackendError<BuildError, PoolError, ConnectionError, QueryError> {
    fn from(value: BuildError) -> Self {
        Self::Build(value)
    }
}

impl From<PoolError> for BackendError<BuildError, PoolError, ConnectionError, QueryError> {
    fn from(value: PoolError) -> Self {
        Self::Pool(value)
    }
}
