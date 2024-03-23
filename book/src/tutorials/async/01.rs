fn main() {}

#[cfg(test)]
mod tests {
    #![allow(dead_code)]

    // import OnceCell
    use tokio::sync::OnceCell;

    async fn get_connection_pool() {
        // add "lazy static"
        static POOL: OnceCell<()> = OnceCell::const_new();
    }
}
