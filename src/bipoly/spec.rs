use crate::circuit::*;
use super::{Polynomial, AlgebraicCircuit};
use num_bigint::BigInt;
use num_traits::One;

pub trait CircuitSpec {
    fn build_golden(&self, ac: &AlgebraicCircuit) -> Polynomial;
    fn modulus(&self, ac: &AlgebraicCircuit) -> Option<BigInt>;
}

pub struct MultiplierSpec;

impl CircuitSpec for MultiplierSpec {
    fn build_golden(&self, ac: &AlgebraicCircuit) -> Polynomial {
        let build_poly = |iter: &mut dyn Iterator<Item = NetId>| -> Polynomial {
            iter.fold(Polynomial::zero(), |acc, net| {
                let var_id = ac.var(net);
                acc * Polynomial::constant(BigInt::from(2)) + Polynomial::var(var_id, BigInt::one())
            })
        };

        let inputs = ac.circuit.inputs();
        let half = inputs.len() / 2;
        
        let poly_a = build_poly(
            &mut inputs[0..half].iter().rev().copied()
        );
        let poly_b = build_poly(
            &mut inputs[half..].iter().rev().copied()
        );
        
        let mut golden = build_poly(
            &mut ac.circuit.outputs().iter().rev().map(|l| l.net())
        );
        for output in ac.circuit.outputs() {
            if output.negative() {
                golden.neg_var(&(&ac.var(output.net())));
            }
        }
        golden - poly_a * poly_b
    }

    fn modulus(&self, ac: &AlgebraicCircuit) -> Option<BigInt> {
        Some(BigInt::from(1) << ac.circuit.outputs().len())
    }
}
