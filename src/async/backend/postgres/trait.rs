use std::{borrow::Cow, fmt::Debug, marker::PhantomData, ops::DerefMut};

use async_trait::async_trait;
use uuid::Uuid;

use crate::{common::statement::postgres, util::get_db_name};

use super::super::error::Error as BackendError;

#[async_trait]
pub(super) trait PostgresBackend<'pool>: Send + Sync + 'static {
    type Connection;
    type PooledConnection: DerefMut<Target = Self::Connection>;
    type Pool;

    type BuildError: Into<
            BackendError<
                Self::BuildError,
                Self::PoolError,
                Self::ConnectionError,
                Self::QueryError,
            >,
        > + Debug;
    type PoolError: Into<
            BackendError<
                Self::BuildError,
                Self::PoolError,
                Self::ConnectionError,
                Self::QueryError,
            >,
        > + Debug;
    type ConnectionError: Into<
            BackendError<
                Self::BuildError,
                Self::PoolError,
                Self::ConnectionError,
                Self::QueryError,
            >,
        > + Debug;
    type QueryError: Into<
            BackendError<
                Self::BuildError,
                Self::PoolError,
                Self::ConnectionError,
                Self::QueryError,
            >,
        > + Debug;

    async fn execute_stmt(
        &self,
        query: &str,
        conn: &mut Self::Connection,
    ) -> Result<(), Self::QueryError>;
    async fn batch_execute_stmt<'a>(
        &self,
        query: impl IntoIterator<Item = Cow<'a, str>> + Send,
        conn: &mut Self::Connection,
    ) -> Result<(), Self::QueryError>;

    async fn get_default_connection(&'pool self)
        -> Result<Self::PooledConnection, Self::PoolError>;
    async fn establish_database_connection(
        &self,
        db_id: Uuid,
    ) -> Result<Self::Connection, Self::ConnectionError>;
    fn put_database_connection(&self, db_id: Uuid, conn: Self::Connection);
    fn get_database_connection(&self, db_id: Uuid) -> Self::Connection;

    async fn get_previous_database_names(
        &self,
        conn: &mut Self::Connection,
    ) -> Result<Vec<String>, Self::QueryError>;
    async fn create_entities(&self, conn: Self::Connection) -> Self::Connection;
    async fn create_connection_pool(&self, db_id: Uuid) -> Result<Self::Pool, Self::BuildError>;

    async fn get_table_names(
        &self,
        privileged_conn: &mut Self::Connection,
    ) -> Result<Vec<String>, Self::QueryError>;

    fn get_drop_previous_databases(&self) -> bool;
}

pub(super) struct PostgresBackendWrapper<'backend, 'pool, B>
where
    B: PostgresBackend<'pool>,
{
    inner: &'backend B,
    _marker: &'pool PhantomData<()>,
}

impl<'backend, 'pool, B> PostgresBackendWrapper<'backend, 'pool, B>
where
    B: PostgresBackend<'pool>,
{
    pub(super) fn new(backend: &'backend B) -> Self {
        Self {
            inner: backend,
            _marker: &PhantomData,
        }
    }
}

impl<'backend, 'pool, B> PostgresBackendWrapper<'backend, 'pool, B>
where
    'backend: 'pool,
    B: PostgresBackend<'pool>,
{
    pub(super) async fn init(
        &'backend self,
    ) -> Result<(), BackendError<B::BuildError, B::PoolError, B::ConnectionError, B::QueryError>>
    {
        // Drop previous databases if needed
        if self.inner.get_drop_previous_databases() {
            // Get connection to default database as privileged user
            let conn = &mut self
                .inner
                .get_default_connection()
                .await
                .map_err(Into::into)?;

            // Get previous database names
            let db_names = self
                .inner
                .get_previous_database_names(conn)
                .await
                .map_err(Into::into)?;

            // Drop databases
            let futures = db_names
                .iter()
                .map(|db_name| async move {
                    let conn = &mut self
                        .inner
                        .get_default_connection()
                        .await
                        .map_err(Into::into)?;
                    self.inner
                        .execute_stmt(postgres::drop_database(db_name.as_str()).as_str(), conn)
                        .await
                        .map_err(Into::into)?;
                    Ok::<
                        _,
                        BackendError<
                            B::BuildError,
                            B::PoolError,
                            B::ConnectionError,
                            B::QueryError,
                        >,
                    >(())
                })
                .collect::<Vec<_>>();
            futures::future::try_join_all(futures).await?;
        }

        Ok(())
    }

    pub(super) async fn create(
        &'backend self,
        db_id: Uuid,
    ) -> Result<B::Pool, BackendError<B::BuildError, B::PoolError, B::ConnectionError, B::QueryError>>
    {
        // Get database name based on UUID
        let db_name = get_db_name(db_id);
        let db_name = db_name.as_str();

        {
            // Get connection to default database as privileged user
            let conn = &mut self
                .inner
                .get_default_connection()
                .await
                .map_err(Into::into)?;

            // Create database
            self.inner
                .execute_stmt(postgres::create_database(db_name).as_str(), conn)
                .await
                .map_err(Into::into)?;

            // Create CRUD role
            self.inner
                .execute_stmt(postgres::create_role(db_name).as_str(), conn)
                .await
                .map_err(Into::into)?;
        }

        {
            // Connect to database as privileged user
            let conn = self
                .inner
                .establish_database_connection(db_id)
                .await
                .map_err(Into::into)?;

            // Create entities
            let mut conn = self.inner.create_entities(conn).await;

            // Grant privileges to CRUD role
            self.inner
                .execute_stmt(
                    postgres::grant_table_privileges(db_name).as_str(),
                    &mut conn,
                )
                .await
                .map_err(Into::into)?;
            self.inner
                .execute_stmt(
                    postgres::grant_sequence_privileges(db_name).as_str(),
                    &mut conn,
                )
                .await
                .map_err(Into::into)?;

            // Store database connection for reuse when cleaning
            self.inner.put_database_connection(db_id, conn);
        }

        // Create connection pool with CRUD role
        let pool = self
            .inner
            .create_connection_pool(db_id)
            .await
            .map_err(Into::into)?;
        Ok(pool)
    }

    pub(super) async fn clean(
        &'backend self,
        db_id: Uuid,
    ) -> Result<(), BackendError<B::BuildError, B::PoolError, B::ConnectionError, B::QueryError>>
    {
        let mut conn = self.inner.get_database_connection(db_id);
        let table_names = self
            .inner
            .get_table_names(&mut conn)
            .await
            .map_err(Into::into)?;
        let stmts = table_names
            .iter()
            .map(|table_name| postgres::truncate_table(table_name.as_str()).into());
        self.inner
            .batch_execute_stmt(stmts, &mut conn)
            .await
            .map_err(Into::into)?;
        self.inner.put_database_connection(db_id, conn);
        Ok(())
    }

    pub(super) async fn drop(
        &'backend self,
        db_id: Uuid,
    ) -> Result<(), BackendError<B::BuildError, B::PoolError, B::ConnectionError, B::QueryError>>
    {
        // Drop privileged connection to database
        {
            self.inner.get_database_connection(db_id);
        }

        // Get database name based on UUID
        let db_name = get_db_name(db_id);
        let db_name = db_name.as_str();

        // Get connection to default database as privileged user
        let conn = &mut self
            .inner
            .get_default_connection()
            .await
            .map_err(Into::into)?;

        // Drop database
        self.inner
            .execute_stmt(postgres::drop_database(db_name).as_str(), conn)
            .await
            .map_err(Into::into)?;

        // Drop CRUD role
        self.inner
            .execute_stmt(postgres::drop_role(db_name).as_str(), conn)
            .await
            .map_err(Into::into)?;

        Ok(())
    }
}
