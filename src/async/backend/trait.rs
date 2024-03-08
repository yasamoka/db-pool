use async_trait::async_trait;
use bb8::{ManageConnection, Pool};
use uuid::Uuid;

#[async_trait]
pub trait AsyncBackend: Sized + Send + Sync + 'static {
    type ConnectionManager: ManageConnection;

    async fn init(&self);
    async fn create(&self, db_id: Uuid) -> Pool<Self::ConnectionManager>;
    async fn clean(&self, db_id: Uuid);
    async fn drop(&self, db_id: Uuid);
}
