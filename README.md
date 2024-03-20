<div align="center">
<img src="./logo.svg" height="150" />
</div>

# db-pool

[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/) [![Book Status](https://github.com/yasamoka/db-pool/workflows/Publish%20Book/badge.svg)](https://yasamoka.github.io/db-pool)

[Book](https://yasamoka.github.io/db-pool)

A thread-safe database pool for running database-tied tests in parallel with:
- Easy setup
- Proper isolation
- Automatic creation, reuse, and cleanup
- Async support

### Databases

- MySQL
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