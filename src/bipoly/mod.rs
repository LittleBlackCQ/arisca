pub mod mono;
pub mod poly;
pub mod spec;
pub mod strategy;
pub mod revsca;
mod ops;
mod debug;

pub use poly::{Polynomial, PolyVerifier};
pub use mono::{Monomial, VarId};
pub use strategy::{Strategy, DefaultStrategy};
pub use revsca::{RevscaStrategy};
