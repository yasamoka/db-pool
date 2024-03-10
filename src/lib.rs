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
mod r#async;
#[cfg(feature = "_sync")]
mod sync;
mod util;

#[cfg(feature = "_async")]
pub use r#async::*;
#[cfg(feature = "_sync")]
pub use sync::*;
