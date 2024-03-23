`db-pool` is a thread-safe database pool for running database-tied tests in parallel with:
- Easy setup
- Proper isolation
- Automatic creation, reuse, and cleanup
- Async support

### Databases

- MySQL (MariaDB)
- PostgreSQL

### Backends & Pools

#### Sync

| Backend         | Pool | Feature         |
| --------------- | ---- | --------------- |
| diesel/mysql    | r2d2 | diesel-mysql    |
| diesel/postgres | r2d2 | diesel-postgres |
| mysql           | r2d2 | mysql           |
| postgres        | r2d2 | postgres        |

#### Async

| Backend               | Pool | Features                                    |
| --------------------- | ---- | ------------------------------------------- |
| diesel-async/mysql    | bb8  | `diesel-async-mysql`, `diesel-async-bb8`    |
| diesel-async/mysql    | mobc | `diesel-async-mysql`, `diesel-async-mobc`   |
| diesel-async/postgres | bb8  | `diesel-async-postgres`, `diesel-async-bb8` |
| diesel-async/postgres | mobc | `diesel-async-postgres`, `diesel-async-bb8` |
| sea-orm/sqlx-mysql    | sqlx | `sea-orm-mysql`                             |
| sea-orm/sqlx-postgres | sqlx | `sea-orm-postgres`                          |
| sqlx/mysql            | sqlx | `sqlx-mysql`                                |
| sqlx/postgres         | sqlx | `sqlx-postgres`                             |
| tokio-postgres        | bb8  | `tokio-postgres`, `tokio-postgres-bb8`      |
| tokio-postgres        | mobc | `tokio-postgres`, `tokio-postgres-mobc`     |
