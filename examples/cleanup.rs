use diesel::{prelude::*, sql_query, PgConnection, RunQueryDsl};

fn main() {
    let conn = &mut PgConnection::establish("postgres://postgres:postgres@localhost:5432").unwrap();

    pg_database::table
        .select(pg_database::datname)
        .load::<String>(conn)
        .unwrap()
        .drain(..)
        .filter(|db_name| db_name.starts_with("db_pool"))
        .map(|db_name| format!("DROP DATABASE {db_name}"))
        .for_each(|stmt| {
            sql_query(stmt).execute(conn).unwrap();
        });
}

table! {
    pg_database (oid) {
        oid -> Int4,
        datname -> Text
    }
}
