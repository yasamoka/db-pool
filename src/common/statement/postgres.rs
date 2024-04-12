#[allow(dead_code)]
pub const GET_DATABASE_NAMES: &str =
    "SELECT datname FROM pg_catalog.pg_database WHERE datname LIKE 'db_pool_%'";

#[allow(dead_code)]
pub const GET_TABLE_NAMES: &str = "SELECT tablename FROM pg_catalog.pg_tables WHERE schemaname != 'pg_catalog' AND schemaname != 'information_schema'";

pub fn create_database(db_name: &str) -> String {
    format!("CREATE DATABASE {db_name}")
}

pub fn create_role(name: &str) -> String {
    format!("CREATE ROLE {name} WITH LOGIN PASSWORD '{name}'")
}

pub fn grant_database_ownership(db_name: &str, role_name: &str) -> String {
    format!("ALTER DATABASE {db_name} OWNER to {role_name}")
}

pub fn grant_restricted_table_privileges(role_name: &str) -> String {
    format!("GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO {role_name}")
}

pub fn grant_restricted_sequence_privileges(role_name: &str) -> String {
    format!("GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO {role_name}")
}

pub fn truncate_table(table_name: &str) -> String {
    format!("TRUNCATE TABLE {table_name} RESTART IDENTITY CASCADE")
}

pub fn drop_database(db_name: &str) -> String {
    format!("DROP DATABASE {db_name}")
}

pub fn drop_role(name: &str) -> String {
    format!("DROP ROLE {name}")
}

#[cfg(test)]
pub(crate) mod tests {
    pub const CREATE_ENTITIES_STATEMENTS: [&str; 2] = [
        "CREATE TABLE book(id SERIAL PRIMARY KEY, title TEXT NOT NULL)",
        "CREATE TABLE dummy(id SERIAL PRIMARY KEY)",
    ];

    pub const DDL_STATEMENTS: [&str; 9] = [
        "CREATE TABLE author()",
        "ALTER TABLE book RENAME TO new_book",
        "ALTER TABLE book ADD description TEXT",
        "ALTER TABLE book ALTER title TYPE TEXT",
        "ALTER TABLE book ALTER title DROP NOT NULL",
        "ALTER TABLE book RENAME title TO new_title",
        "ALTER TABLE book DROP title",
        "TRUNCATE TABLE book",
        "DROP TABLE book",
    ];

    pub const DML_STATEMENTS: [&str; 4] = [
        "SELECT * FROM book",
        "INSERT INTO book (title) VALUES ('Title')",
        "UPDATE book SET title = 'Title 2' WHERE id = 1",
        "DELETE FROM book WHERE id = 1",
    ];
}
