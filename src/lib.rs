#![forbid(unsafe_code)]
#![warn(
    clippy::cargo,
    clippy::complexity,
    clippy::correctness,
    clippy::pedantic,
    clippy::perf,
    clippy::style,
    clippy::suspicious,
    clippy::unwrap_used
)]
#![allow(
    clippy::module_name_repetitions,
    clippy::module_inception,
    clippy::missing_errors_doc
)]

mod common;

#[cfg(feature = "_async")]
pub mod r#async;
#[cfg(feature = "_sync")]
pub mod sync;
mod util;

#[allow(unused_imports)]
pub use common::config::*;

#[cfg(test)]
mod tests {
    use tokio::sync::RwLock;

    pub static DROP_LOCK: RwLock<()> = RwLock::const_new(());
}
