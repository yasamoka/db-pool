use uuid::Uuid;

pub fn get_db_name(id: Uuid) -> String {
    format!("db_pool_{}", id.to_string().replace("-", "_"))
}
