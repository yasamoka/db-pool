`db-pool` is a thread-safe database pool for running database-tied tests in parallel with:
- Easy setup
- Proper isolation
- Automatic creation, reuse, and cleanup
- Async support

### Databases

- MySQL (MariaDB)
- PostgreSQL

### Backends

#### Sync

| Backend         | Feature         |
| --------------- | --------------- |
| diesel/mysql    | diesel-mysql    |
| diesel/postgres | diesel-postgres |
| mysql           | mysql           |
| postgres        | postgres        |

#### Async

| Backend               | Feature               |
| --------------------- | --------------------- |
| diesel-async/mysql    | diesel-async-mysql    |
| diesel-async/postgres | diesel-async-postgres |
| sea-orm/sqlx-mysql    | sea-orm-mysql         |
| sea-orm/sqlx-postgres | sea-orm-postgres      |
| sqlx/mysql            | sqlx-mysql            |
| sqlx/postgres         | sqlx-postgres         |
| tokio-postgres        | tokio-postgres        |