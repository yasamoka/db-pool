pub const USE_DEFAULT_DATABASE: &str = "USE information_schema";

pub const GET_DATABASE_NAMES: &str =
    "SELECT schema_name FROM information_schema.schemata WHERE schema_name LIKE 'db_pool_%';";

pub const TURN_OFF_FOREIGN_KEY_CHECKS: &str = "SET FOREIGN_KEY_CHECKS = 0";
pub const TURN_ON_FOREIGN_KEY_CHECKS: &str = "SET FOREIGN_KEY_CHECKS = 1";

pub fn create_database(db_name: &str) -> String {
    format!("CREATE DATABASE {db_name}")
}

pub fn create_user(name: &str, host: &str) -> String {
    format!("CREATE USER {name}@{host} IDENTIFIED BY '{name}'")
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

pub fn drop_database(db_name: &str) -> String {
    format!("DROP DATABASE {db_name}")
}

pub fn drop_user(name: &str, host: &str) -> String {
    format!("DROP USER {name}@{host}")
}
