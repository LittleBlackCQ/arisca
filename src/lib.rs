pub mod aiger;
pub mod bipoly;
pub mod circuit;
pub mod config;
pub mod json;
pub mod portfolio;
pub mod verifier;

use std::error::Error;
pub type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync + 'static>>;
