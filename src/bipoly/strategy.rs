use super::Polynomial;
use super::poly::AlgebraicCircuit;
use crate::circuit::*;

use std::collections::HashMap;

pub trait Strategy {
    fn apply(&self, ac: &AlgebraicCircuit, poly_map: &mut HashMap<NetId, Polynomial>);
}

pub struct DefaultStrategy;
impl Strategy for DefaultStrategy {
    fn apply(&self, _ac: &AlgebraicCircuit, _poly_map: &mut HashMap<NetId, Polynomial>) {}
}
