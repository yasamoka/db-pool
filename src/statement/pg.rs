#[allow(dead_code)]
pub const GET_TABLE_NAMES: &'static str = "SELECT tablename FROM pg_catalog.pg_tables WHERE schemaname != 'pg_catalog' AND schemaname != 'information_schema'";

pub fn create_database(db_name: &str) -> String {
    format!("CREATE DATABASE {db_name}")
}

pub fn create_role(name: &str) -> String {
    format!("CREATE ROLE {name} WITH LOGIN")
}

pub fn grant_table_privileges(role_name: &str) -> String {
    format!("GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO {role_name}")
}

pub fn grant_sequence_privileges(role_name: &str) -> String {
    format!("GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO {role_name}")
}

pub fn truncate_table(table_name: &str) -> String {
    format!("TRUNCATE TABLE {table_name} RESTART IDENTITY CASCADE")
}

pub fn terminate_database_connections(db_name: &str) -> String {
    format!("SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = {db_name} AND leader_pid IS NULL")
}

pub fn drop_database(db_name: &str) -> String {
    format!("DROP DATABASE {}", db_name)
}

pub fn drop_role(name: &str) -> String {
    format!("DROP ROLE {}", name)
}
