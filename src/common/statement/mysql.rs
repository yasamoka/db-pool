#[allow(dead_code)]
pub const GET_DATABASE_NAMES: &str =
    "SELECT schema_name FROM information_schema.schemata WHERE schema_name LIKE 'db_pool_%';";

pub const TURN_OFF_FOREIGN_KEY_CHECKS: &str = "SET FOREIGN_KEY_CHECKS = 0";
pub const TURN_ON_FOREIGN_KEY_CHECKS: &str = "SET FOREIGN_KEY_CHECKS = 1";

pub const USE_DEFAULT_DATABASE: &str = "USE information_schema";

pub fn create_database(db_name: &str) -> String {
    format!("CREATE DATABASE {db_name}")
}

pub fn create_user(name: &str, host: &str) -> String {
    format!("CREATE USER {name}@{host} IDENTIFIED BY '{name}'")
}

pub fn use_database(db_name: &str) -> String {
    format!("USE {db_name}")
}

pub fn grant_all_privileges(db_name: &str, host: &str) -> String {
    format!("GRANT ALL PRIVILEGES ON {db_name}.* TO {db_name}@{host}")
}

pub fn grant_restricted_privileges(db_name: &str, host: &str) -> String {
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

#[cfg(test)]
pub(crate) mod tests {
    pub const CREATE_ENTITIES_STATEMENTS: [&str; 2] = [
        "CREATE TABLE book(id INTEGER PRIMARY KEY AUTO_INCREMENT, title TEXT NOT NULL)",
        "CREATE TABLE dummy(id INTEGER PRIMARY KEY AUTO_INCREMENT)",
    ];

    pub const DDL_STATEMENTS: [&str; 11] = [
        "CREATE TABLE author(id INTEGER)",
        "ALTER TABLE book RENAME TO new_book",
        "ALTER TABLE book ADD description TEXT",
        "ALTER TABLE book MODIFY title TEXT",
        "ALTER TABLE book MODIFY title TEXT NOT NULL",
        "ALTER TABLE book RENAME COLUMN title TO new_title",
        "ALTER TABLE book CHANGE title new_title TEXT",
        "ALTER TABLE book CHANGE title new_title TEXT NOT NULL",
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
