pub const USE_DEFAULT_DATABASE: &str = "USE information_schema";
pub const TURN_OFF_FOREIGN_KEY_CHECKS: &str = "SET FOREIGN_KEY_CHECKS = 0";
pub const TURN_ON_FOREIGN_KEY_CHECKS: &str = "SET FOREIGN_KEY_CHECKS = 1";

pub fn create_database(db_name: &str) -> String {
    format!("CREATE DATABASE {db_name}")
}

pub fn create_user(name: &str, host: &str) -> String {
    format!("CREATE USER {name}@{host} IDENTIFIED BY ''")
}

pub fn use_database(db_name: &str) -> String {
    format!("USE {db_name}")
}

pub fn grant_privileges(db_name: &str, host: &str) -> String {
    format!("GRANT SELECT, INSERT, UPDATE, DELETE ON {db_name}.* TO {db_name}@{host}")
}

#[allow(dead_code)]
pub fn get_table_names(db_name: &str) -> String {
    format!("SELECT table_name FROM information_schema.tables WHERE table_schema = '{db_name}'")
}

pub fn truncate_table(table_name: &str, db_name: &str) -> String {
    format!("TRUNCATE TABLE {db_name}.{table_name}")
}

#[allow(dead_code)]
pub fn get_database_connection_ids(db_name: &str, host: &str) -> String {
    format!("SELECT id FROM information_schema.processlist WHERE user = {db_name}@{host}")
}

pub fn terminate_database_connection(id: i64) -> String {
    format!("KILL {}", id)
}

pub fn drop_database(db_name: &str) -> String {
    format!("DROP DATABASE {}", db_name)
}

pub fn drop_user(name: &str, host: &str) -> String {
    format!("DROP USER {name}@{host}")
}
