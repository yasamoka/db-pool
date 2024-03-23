fn main() {}

#[cfg(test)]
mod tests {
    #![allow(dead_code, unused_variables)]

    // import privileged configuration
    use db_pool::PrivilegedPostgresConfig;
    // import dotenvy
    use dotenvy::dotenv;
    use tokio::sync::OnceCell;

    async fn get_connection_pool() {
        static POOL: OnceCell<()> = OnceCell::const_new();

        let db_pool = POOL
            .get_or_init(|| async {
                // load environment variables from .env
                dotenv().ok();

                // create privileged configuration from environment variables
                let config = PrivilegedPostgresConfig::from_env().unwrap();
            })
            .await;
    }
}
