pub mod mono;
pub mod poly;
mod ops;
mod debug;

pub use poly::{Polynomial};
pub use mono::{Monomial, VarId, Term};
