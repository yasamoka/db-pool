use std::{
    borrow::Cow,
    fmt::Debug,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use async_trait::async_trait;
use uuid::Uuid;

use crate::{common::statement::mysql, util::get_db_name};

use super::super::error::Error as BackendError;

#[async_trait]
pub(super) trait MySQLBackend<'pool>: Send + Sync + 'static {
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

    async fn get_connection(&'pool self) -> Result<Self::PooledConnection, Self::PoolError>;

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

    fn get_host(&self) -> &str;

    async fn get_previous_database_names(
        &self,
        conn: &mut Self::Connection,
    ) -> Result<Vec<String>, Self::QueryError>;
    async fn create_entities(&self, db_name: &str) -> Result<(), Self::ConnectionError>;
    async fn create_connection_pool(&self, db_id: Uuid) -> Result<Self::Pool, Self::BuildError>;

    async fn get_table_names(
        &self,
        db_name: &str,
        conn: &mut Self::Connection,
    ) -> Result<Vec<String>, Self::QueryError>;

    fn get_drop_previous_databases(&self) -> bool;
}

pub(super) struct MySQLBackendWrapper<'backend, 'pool, B>
where
    B: MySQLBackend<'pool>,
{
    inner: &'backend B,
    _marker: &'pool PhantomData<()>,
}

impl<'backend, 'pool, B> MySQLBackendWrapper<'backend, 'pool, B>
where
    B: MySQLBackend<'pool>,
{
    pub(super) fn new(backend: &'backend B) -> Self {
        Self {
            inner: backend,
            _marker: &PhantomData,
        }
    }
}

impl<'backend, 'pool, B> Deref for MySQLBackendWrapper<'backend, 'pool, B>
where
    B: MySQLBackend<'pool>,
{
    type Target = B;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl<'backend, 'pool, B> MySQLBackendWrapper<'backend, 'pool, B>
where
    'backend: 'pool,
    B: MySQLBackend<'pool>,
{
    pub(super) async fn init(
        &'backend self,
    ) -> Result<(), BackendError<B::BuildError, B::PoolError, B::ConnectionError, B::QueryError>>
    {
        // Drop previous databases if needed
        if self.get_drop_previous_databases() {
            // Get privileged connection
            let conn = &mut self.get_connection().await.map_err(Into::into)?;

            // Get previous database names
            self.execute_stmt(mysql::USE_DEFAULT_DATABASE, conn)
                .await
                .map_err(Into::into)?;
            let mut db_names = self
                .get_previous_database_names(conn)
                .await
                .map_err(Into::into)?;

            // Drop databases
            let futures = db_names
                .drain(..)
                .map(|db_name| async move {
                    let conn = &mut self.get_connection().await.map_err(Into::into)?;
                    self.execute_stmt(mysql::drop_database(db_name.as_str()).as_str(), conn)
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
        db_id: uuid::Uuid,
    ) -> Result<B::Pool, BackendError<B::BuildError, B::PoolError, B::ConnectionError, B::QueryError>>
    {
        // Get database name based on UUID
        let db_name = get_db_name(db_id);
        let db_name = db_name.as_str();

        let host = self.get_host();

        // Get privileged connection
        let conn = &mut self.get_connection().await.map_err(Into::into)?;

        // Create database
        self.execute_stmt(mysql::create_database(db_name).as_str(), conn)
            .await
            .map_err(Into::into)?;

        // Create CRUD user
        self.execute_stmt(mysql::create_user(db_name, host).as_str(), conn)
            .await
            .map_err(Into::into)?;

        // Create entities
        self.execute_stmt(mysql::use_database(db_name).as_str(), conn)
            .await
            .map_err(Into::into)?;
        self.create_entities(db_name).await.map_err(Into::into)?;
        self.execute_stmt(mysql::USE_DEFAULT_DATABASE, conn)
            .await
            .map_err(Into::into)?;

        // Grant privileges to CRUD role
        self.execute_stmt(mysql::grant_privileges(db_name, host).as_str(), conn)
            .await
            .map_err(Into::into)?;

        // Create connection pool with CRUD role
        let pool = self
            .create_connection_pool(db_id)
            .await
            .map_err(Into::into)?;
        Ok(pool)
    }

    pub(super) async fn clean(
        &'backend self,
        db_id: uuid::Uuid,
    ) -> Result<(), BackendError<B::BuildError, B::PoolError, B::ConnectionError, B::QueryError>>
    {
        let db_name = get_db_name(db_id);
        let db_name = db_name.as_str();

        let conn = &mut self.get_connection().await.map_err(Into::into)?;

        let table_names = self
            .get_table_names(db_name, conn)
            .await
            .map_err(Into::into)?;
        let stmts = table_names
            .iter()
            .map(|table_name| mysql::truncate_table(table_name.as_str(), db_name).into());

        self.execute_stmt(mysql::TURN_OFF_FOREIGN_KEY_CHECKS, conn)
            .await
            .map_err(Into::into)?;
        self.batch_execute_stmt(stmts, conn)
            .await
            .map_err(Into::into)?;
        self.execute_stmt(mysql::TURN_ON_FOREIGN_KEY_CHECKS, conn)
            .await
            .map_err(Into::into)?;
        Ok(())
    }

    pub(super) async fn drop(
        &'backend self,
        db_id: uuid::Uuid,
    ) -> Result<(), BackendError<B::BuildError, B::PoolError, B::ConnectionError, B::QueryError>>
    {
        // Get database name based on UUID
        let db_name = get_db_name(db_id);
        let db_name = db_name.as_str();

        let host = self.get_host();

        // Get privileged connection
        let conn = &mut self.get_connection().await.map_err(Into::into)?;

        // Drop database
        self.execute_stmt(mysql::drop_database(db_name).as_str(), conn)
            .await
            .map_err(Into::into)?;

        // Drop CRUD role
        self.execute_stmt(mysql::drop_user(db_name, host).as_str(), conn)
            .await
            .map_err(Into::into)?;

        Ok(())
    }
}
