fn main() {}

#[cfg(test)]
mod tests {
    #![allow(dead_code)]

    // import OnceLock
    use std::sync::OnceLock;

    fn get_connection_pool() {
        // add "lazy static"
        static POOL: OnceLock<()> = OnceLock::new();
    }
}
