use crate::circuit::*;
use super::Polynomial;
use num_bigint::BigInt;
use num_traits::One;

pub trait CircuitSpec {
    fn build_golden(&self, circuit: &Circuit) -> Polynomial;
    fn modulus(&self, circuit: &Circuit) -> Option<BigInt>;
}

pub struct MultiplierSpec;

impl CircuitSpec for MultiplierSpec {
    fn build_golden(&self, circuit: &Circuit) -> Polynomial {
        let build_poly = |iter: &mut dyn Iterator<Item = NetId>| -> Polynomial {
            iter.fold(Polynomial::zero(), |acc, net| {
                let var_id = circuit.get_topo_index(net) as u32;
                acc * Polynomial::constant(BigInt::from(2)) + Polynomial::var(var_id, BigInt::one())
            })
        };

        let inputs = circuit.inputs();
        let half = inputs.len() / 2;
        
        let poly_a = build_poly(
            &mut inputs[0..half].iter().rev().copied()
        );
        let poly_b = build_poly(
            &mut inputs[half..].iter().rev().copied()
        );
        
        let mut golden = build_poly(
            &mut circuit.outputs().iter().rev().map(|l| l.net())
        );
        for output in circuit.outputs() {
            if output.negative() {
                golden.neg_var(&(circuit.get_topo_index(output.net()) as u32));
            }
        }
        golden - poly_a * poly_b
    }

    fn modulus(&self, circuit: &Circuit) -> Option<BigInt> {
        Some(BigInt::from(1) << circuit.outputs().len())
    }
}
