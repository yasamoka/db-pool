#[cfg(test)]
mod tests {
    use std::sync::OnceLock;

    // import privileged configuration
    use db_pool::PrivilegedPostgresConfig;
    // import dotenvy
    use dotenvy::dotenv;

    fn get_connection_pool() {
        static POOL: OnceLock<()> = OnceLock::new();
        let db_pool = POOL.get_or_init(|| {
            // load environment variables from .env
            dotenv().ok();
            // create privileged configuration from environment variables
            let config = PrivilegedPostgresConfig::from_env().unwrap();
        });
    }
}
