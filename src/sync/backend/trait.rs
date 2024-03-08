use r2d2::{ManageConnection, Pool};
use uuid::Uuid;

pub trait Backend: Sized + Send + Sync + 'static {
    type ConnectionManager: ManageConnection;

    fn init(&self);
    fn create(&self, db_id: Uuid) -> Pool<Self::ConnectionManager>;
    fn clean(&self, db_id: Uuid);
    fn drop(&self, db_id: Uuid);
}
