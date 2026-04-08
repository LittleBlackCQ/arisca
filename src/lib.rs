pub mod bipoly;
pub mod circuit;
pub mod verifier;
pub mod aiger;
pub mod config;
pub mod portfolio;
pub mod json;

use std::error::Error;
pub type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync + 'static>>;