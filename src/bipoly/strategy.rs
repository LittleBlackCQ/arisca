use super::{Polynomial, AlgebraicCircuit, VarId};
use crate::circuit::*;

use std::collections::HashMap;

pub trait Strategy {
    fn init(&mut self, ac: &AlgebraicCircuit);

    fn gen_var_map(&self, circuit: &Circuit) -> Vec<VarId>;

    fn pre_reduce(&mut self, ac: &AlgebraicCircuit, poly_map: &mut HashMap<VarId, Polynomial>);

    fn init_order(&mut self, ac: &AlgebraicCircuit);

    fn next_reduction_var(
        &mut self, 
        current_poly: &Polynomial, 
        poly_map: &HashMap<VarId, Polynomial>
    ) -> ReductionAction;

    fn post_reduce(poly: &mut Polynomial);
}

#[derive(Default)]
pub struct DefaultStrategy {
    order_iter: Option<std::vec::IntoIter<VarId>>
}

impl Strategy for DefaultStrategy {
    fn init(&mut self, _ac: &AlgebraicCircuit) {}

    fn gen_var_map(&self, circuit: &Circuit) -> Vec<VarId> {
        (0..circuit.nets().len() as i32).collect()
    }

    fn pre_reduce(&mut self, _ac: &AlgebraicCircuit, _poly_map: &mut HashMap<VarId, Polynomial>) {}

    fn init_order(&mut self, ac: &AlgebraicCircuit) {
        let vars: Vec<VarId> = (0..ac.circuit.nets().len())
            .map(|i| ac.var(i))
            .rev()
            .collect();

        self.order_iter = Some(vars.into_iter());
    }

    fn next_reduction_var(
        &mut self, 
        _current_poly: &Polynomial, 
        _poly_map: &HashMap<VarId, Polynomial>
    ) -> ReductionAction {
        let next_var = self.order_iter.as_mut().and_then(|iter| iter.next());
        match next_var {
            Some(var) => ReductionAction::Reduce(var),
            None => ReductionAction::Stop,
        }
    }

    fn post_reduce(_poly: &mut Polynomial) {}
}

pub enum ReductionAction {
    Reduce(VarId),
    Replace(Polynomial, VarId),
    Stop
}