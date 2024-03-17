<img src="./logo.svg" height="150" />

# db-pool

[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)

A thread-safe database pool for running database-tied tests in parallel with:
- Easy setup
- Proper isolation
- Automatic creation, reuse, and cleanup

### Databases

- PostgreSQL
- MySQL

### Backends

#### Sync

| Backend         | Feature         |
| --------------- | --------------- |
| diesel/postgres | diesel-postgres |
| diesel/mysql    | diesel-mysql    |
| postgres        | postgres        |
| mysql           | mysql           |

#### Async

| Backend               | Feature               |
| --------------------- | --------------------- |
| diesel-async/postgres | diesel-async-postgres |
| diesel-async/mysql    | diesel-async-mysql    |
| sea-orm/sqlx-postgres | sea-orm-postgres      |
| sea-orm/sqlx-mysql    | sea-orm-mysql         |
| sqlx/postgres         | sqlx-postgres         |
| sqlx/mysql            | sqlx-mysql            |
| tokio-postgres        | tokio-postgres        |