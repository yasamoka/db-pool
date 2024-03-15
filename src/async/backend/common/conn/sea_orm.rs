use std::ops::{Deref, DerefMut};

use sea_orm::DatabaseConnection;

pub struct PooledConnection(DatabaseConnection);

impl From<DatabaseConnection> for PooledConnection {
    fn from(value: DatabaseConnection) -> Self {
        Self(value)
    }
}

impl Deref for PooledConnection {
    type Target = DatabaseConnection;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for PooledConnection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
