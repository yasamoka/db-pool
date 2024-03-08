#![warn(clippy::cargo, clippy::pedantic)]
#![allow(clippy::module_name_repetitions, clippy::module_inception)]

#[cfg(feature = "_async")]
mod r#async;
mod statement;
#[cfg(feature = "_sync")]
mod sync;
mod util;

#[cfg(feature = "_async")]
pub use r#async::*;
#[cfg(feature = "_sync")]
pub use sync::*;
