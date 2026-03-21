//! App module design
//! - `system`: app lifecycle orchestration
//! - `struct`: app-local data structures
//! - `config`: app-level constants
//! - `utils`: helper routines used by system

pub mod config;
pub mod r#struct;
pub mod system;
pub mod utils;

pub use system::run;
