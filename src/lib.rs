/// qrtools_lib — public library surface
///
/// Exposes service modules so they can be unit-tested without starting an HTTP
/// server and used as a library by downstream crates.

pub mod config;
pub mod error;
pub mod middleware;
pub mod services;
pub mod state;
