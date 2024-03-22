#[cfg(test)]
mod tests {
    use std::sync::OnceLock;

    fn get_connection_pool() {
        // add "lazy static"
        static POOL: OnceLock<()> = OnceLock::new();
    }
}
