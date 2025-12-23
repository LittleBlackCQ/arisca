use bitvec::prelude::*;
use super::Circuit;

pub struct Simulator<'a> {
    circuit: &'a Circuit,
    net_values: BitVec,
}

impl<'a> Simulator<'a> {
    pub fn new(circuit: &'a Circuit) -> Self {
        Simulator {
            circuit,
            net_values: BitVec::repeat(false, circuit.nets().len()),
        }
    }

    pub fn set_inputs(&mut self, values: &[bool]) {
        assert_eq!(values.len(), self.circuit.inputs.len());
        for (i, &net) in self.circuit.inputs.iter().enumerate() {
            self.net_values.set(net, values[i]);
        }
    }

    pub fn step(&mut self) {
        for net in self.circuit.topology_order() {
            if let Some(node) = self.circuit.nets()[net].driver() {
                let node = &self.circuit.nodes()[node];
                let ins: Vec<bool> = node.inputs().iter()
                    .map(|lit| {
                        let v = self.net_values[lit.net()];
                        if lit.negative() { !v } else { v }
                    })
                    .collect();
                let out_val = node.gate().logic(&ins);
                for (i, &net) in node.outputs().iter().enumerate() {
                    self.net_values.set(net, out_val[i]);
                }
            }
        }
    }
    
    pub fn outputs(&self) -> Vec<bool> {
        self.circuit.outputs().iter()
            .map(|lit| {
                let v = self.net_values[lit.net()];
                if lit.negative() { !v } else { v }
            })
            .collect()
    }

    pub fn run(&mut self, inputs: &[bool]) -> Vec<bool> {
        self.set_inputs(inputs);
        self.step();
        self.outputs()
    }
    
    pub fn eval(circuit: &'a Circuit, inputs: &[bool]) -> Vec<bool> {
        let mut sim = Simulator::new(circuit);
        sim.run(inputs)
    }

    pub fn get_tt(circuit: &'a Circuit) -> Vec<Vec<bool>> {
        let n = circuit.inputs().len();
        assert!(n <= usize::BITS as usize, "too many inputs");

        let rows = 1usize << n;
        let mut table = Vec::with_capacity(rows);
        
        let mut sim = Simulator::new(circuit); 

        for mask in 0..rows {
            let mut inputs = Vec::with_capacity(n);
            for i in 0..n {
                inputs.push(((mask >> i) & 1) != 0);
            }

            table.push(sim.run(&inputs));
        }
        table
    }
}
