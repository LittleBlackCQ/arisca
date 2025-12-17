use mulsca::bipoly::poly::Polynomial;
use mulsca::bipoly::mono::{Monomial, VarId};
use mulsca::circuit::{gate::Gate, Circuit};
use std::fs::File;
use env_logger::Env;

fn main() {
    let file = File::create("run.log").unwrap();

    env_logger::Builder::from_env(Env::default())
        .format_timestamp(None)
        .format_target(false)
        .target(env_logger::Target::Pipe(Box::new(file)))
        .init();

    let path = "/home/likezhi/mulsca/testbench/33.aig";
    let circuit = Circuit::from_aig(path);
    let mut input1 = Polynomial::zero();
    for idx in (0..(circuit.inputs().len() / 2)).rev() {
        if let Some(input) = circuit.inputs().get(idx) {
            let input = u32::try_from(*input).expect("net index too large");
            input1 *= Polynomial::constant(2);
            input1 += Polynomial::var(input, 1);
        }
    }
    let mut input2 = Polynomial::zero();
    for idx in ((circuit.inputs().len() / 2)..circuit.inputs().len()).rev() {
        if let Some(input) = circuit.inputs().get(idx) {
            let input = u32::try_from(*input).expect("net index too large");
            input2 *= Polynomial::constant(2);
            input2 += Polynomial::var(input, 1);
        }
    }

    let mut golden = Polynomial::zero();
    for output in circuit.outputs().iter().rev() {
        let output = u32::try_from(output.net()).expect("net index too large");
        golden *= Polynomial::constant(2);
        golden += Polynomial::var(output, 1);
    }

    println!("{:?}", golden);
    golden -= input1 * input2;
    println!("{:?}", golden);

    let res = circuit.check_poly(&golden);
    println!("{}", res);
}
