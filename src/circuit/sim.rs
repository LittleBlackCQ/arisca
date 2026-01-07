use bitvec::prelude::*;
use super::*;

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

    fn compute(
        &mut self, 
        input_nets: &[NetId], 
        input_values: &[bool], 
        internal_nets: &[NetId], 
        targets: &[NetId]
    ) -> Vec<bool> {
        debug_assert_eq!(input_nets.len(), input_values.len());

        for (i, &net) in input_nets.iter().enumerate() {
            self.net_values.set(net, input_values[i]);
        }

        for &net in internal_nets {
            if let Some(node_id) = self.circuit.nets_at(net).driver() {
                let node = self.circuit.nodes_at(node_id);
                // Collect inputs for the gate from current state
                let gate_inputs: Vec<bool> = node.inputs().iter()
                    .map(|lit| {
                        let v = self.net_values[lit.net()];
                        if lit.negative() { !v } else { v }
                    })
                    .collect();
                
                let output_values = node.gate().logic(&gate_inputs);
                
                // Update driver's output nets
                for (i, &out_net) in node.outputs().iter().enumerate() {
                    self.net_values.set(out_net, output_values[i]);
                }
            }
        }

        targets.iter().map(|&net| self.net_values[net]).collect()
    }

    fn run_full(&mut self, inputs: &[bool]) -> Vec<bool> {
        // Run simulation updating all nets in topological order
        self.compute(self.circuit.inputs(), inputs, &self.circuit.topology_nets(), &[]);

        // Extract final circuit outputs handling literal negation
        self.circuit.outputs().iter()
            .map(|lit| {
                let v = self.net_values[lit.net()];
                if lit.negative() { !v } else { v }
            })
            .collect()
    }

    pub fn eval(&mut self, inputs: &[bool]) -> Vec<bool> {
        self.run_full(inputs)
    }

    pub fn get_partial_tt(
        &mut self,
        inputs_nets: &[NetId],
        internal_nets: &[NetId], 
        targets: &[NetId]
    ) -> Vec<Vec<bool>> {
        let k = inputs_nets.len();
        assert!(k <= 16, "too many inputs for full TT");

        let rows = 1 << k;
        let mut table = Vec::with_capacity(rows);

        // internal nets should be sorted by topology order
        for mask in 0..rows {
            let inputs: Vec<bool> = (0..k).map(|i| (mask >> i) & 1 != 0).collect();
            table.push(self.compute(inputs_nets, &inputs, internal_nets, targets));
        }
        table
    }

    pub fn get_tt(&mut self) -> Vec<Vec<bool>> {
        let n = self.circuit.inputs().len();
        assert!(n <= 16, "too many inputs for full TT");

        let rows = 1 << n;
        let mut table = Vec::with_capacity(rows);

        for mask in 0..rows {
            let inputs: Vec<bool> = (0..n).map(|i| (mask >> i) & 1 != 0).collect();
            table.push(self.run_full(&inputs));
        }
        table
    }

    pub fn get_tt_transposed(&mut self) -> Vec<Vec<bool>> { 
        let tt = self.get_tt();
        (0..tt[0].len()).map(|i| {
            tt.iter().map(|row| row[i].clone()).collect()
        }).collect()
    }

    pub fn compute_tt(circuit: &'a Circuit) -> Vec<Vec<bool>> {
        let mut sim = Simulator::new(circuit);
        sim.get_tt()
    }

    pub fn compute_tt_transposed(circuit: &'a Circuit) -> Vec<Vec<bool>> {
        let mut sim = Simulator::new(circuit);
        sim.get_tt_transposed()
    }
}
