use serde_json::json;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::{
    json::ToJson,
    aiger::{AigData, ToAig}
};
use super::{Circuit, NetLit, Gate};


impl ToAig for Circuit {
    fn get_aig_data(&self) -> AigData {
        let mut and_gates = Vec::new();
        let mut net_to_lit = vec![0usize; self.nets.len()];
        
        let mut input_lits = Vec::new();
        for (i, &net_id) in self.inputs.iter().enumerate() {
            let var_id = i + 1;
            let lit = var_id * 2;
            net_to_lit[net_id] = lit;
            input_lits.push(lit);
        }

        let mut next_var = self.inputs.len();

        let get_lit = |nl: &NetLit, mapping: &[usize]| -> usize {
            let base = mapping[nl.net()];
            if nl.negative() { base ^ 1 } else { base }
        };

        let add_and = |in1: usize, in2: usize, gates: &mut Vec<(usize, usize, usize)>, var_cnt: &mut usize| -> usize {
            *var_cnt += 1;
            let out_lit = *var_cnt * 2;
            gates.push((out_lit, in1, in2));
            out_lit
        };

        for node in &self.nodes {
            match node.gate() {
                Gate::And => {
                    let l1 = get_lit(&node.inputs()[0], &net_to_lit);
                    let l2 = get_lit(&node.inputs()[1], &net_to_lit);
                    let out = add_and(l1, l2, &mut and_gates, &mut next_var);
                    net_to_lit[node.outputs()[0]] = out;
                }
                Gate::Or => {
                    // a | b = !(!a & !b)
                    let l1 = get_lit(&node.inputs()[0], &net_to_lit) ^ 1;
                    let l2 = get_lit(&node.inputs()[1], &net_to_lit) ^ 1;
                    let out = add_and(l1, l2, &mut and_gates, &mut next_var) ^ 1;
                    net_to_lit[node.outputs()[0]] = out;
                }
                Gate::Xor => {
                    // a ^ b = (!a & b) | (a & !b)
                    let a = get_lit(&node.inputs()[0], &net_to_lit);
                    let b = get_lit(&node.inputs()[1], &net_to_lit);
                    let t1 = add_and(a ^ 1, b, &mut and_gates, &mut next_var);
                    let t2 = add_and(a, b ^ 1, &mut and_gates, &mut next_var);

                    let out = add_and(t1 ^ 1, t2 ^ 1, &mut and_gates, &mut next_var) ^ 1;
                    net_to_lit[node.outputs()[0]] = out;
                }
                Gate::Xor3 => { 
                    // a ^ b ^ c = (a ^ b) ^ c
                    let a = get_lit(&node.inputs()[0], &net_to_lit);
                    let b = get_lit(&node.inputs()[1], &net_to_lit);
                    let c = get_lit(&node.inputs()[2], &net_to_lit);
                    
                    // First compute a ^ b
                    let ab_t1 = add_and(a ^ 1, b, &mut and_gates, &mut next_var);
                    let ab_t2 = add_and(a, b ^ 1, &mut and_gates, &mut next_var);
                    let ab_xor = add_and(ab_t1 ^ 1, ab_t2 ^ 1, &mut and_gates, &mut next_var) ^ 1;
                    
                    // Then compute (a ^ b) ^ c
                    let abc_t1 = add_and(ab_xor ^ 1, c, &mut and_gates, &mut next_var);
                    let abc_t2 = add_and(ab_xor, c ^ 1, &mut and_gates, &mut next_var);
                    let out = add_and(abc_t1 ^ 1, abc_t2 ^ 1, &mut and_gates, &mut next_var) ^ 1;
                    
                    net_to_lit[node.outputs()[0]] = out;
                }
                Gate::Maj => {
                    // Maj(a, b, c) = (a&b) | (b&c) | (a&c)
                    let a = get_lit(&node.inputs()[0], &net_to_lit);
                    let b = get_lit(&node.inputs()[1], &net_to_lit);
                    let c = get_lit(&node.inputs()[2], &net_to_lit);
                    let ab = add_and(a, b, &mut and_gates, &mut next_var);
                    let bc = add_and(b, c, &mut and_gates, &mut next_var);
                    let ac = add_and(a, c, &mut and_gates, &mut next_var);
                    let t = add_and(ab ^ 1, bc ^ 1, &mut and_gates, &mut next_var) ^ 1;
                    let out = add_and(t ^ 1, ac ^ 1, &mut and_gates, &mut next_var) ^ 1;
                    net_to_lit[node.outputs()[0]] = out;
                }
                Gate::HalfAdder => {
                    let a = get_lit(&node.inputs()[0], &net_to_lit);
                    let b = get_lit(&node.inputs()[1], &net_to_lit);
                    // Sum = a ^ b
                    let t1 = add_and(a ^ 1, b, &mut and_gates, &mut next_var);
                    let t2 = add_and(a, b ^ 1, &mut and_gates, &mut next_var);
                    let sum = add_and(t1 ^ 1, t2 ^ 1, &mut and_gates, &mut next_var) ^ 1;
                    // Carry = a & b
                    let carry = add_and(a, b, &mut and_gates, &mut next_var);
                    
                    net_to_lit[node.outputs()[0]] = carry;
                    net_to_lit[node.outputs()[1]] = sum;
                }
                Gate::FullAdder => {
                    let a = get_lit(&node.inputs()[0], &net_to_lit);
                    let b = get_lit(&node.inputs()[1], &net_to_lit);
                    let cin = get_lit(&node.inputs()[2], &net_to_lit);

                    // Sum = a ^ b ^ cin
                    let x1_t1 = add_and(a ^ 1, b, &mut and_gates, &mut next_var);
                    let x1_t2 = add_and(a, b ^ 1, &mut and_gates, &mut next_var);
                    let x1 = add_and(x1_t1 ^ 1, x1_t2 ^ 1, &mut and_gates, &mut next_var) ^ 1;
                    
                    let sum_t1 = add_and(x1 ^ 1, cin, &mut and_gates, &mut next_var);
                    let sum_t2 = add_and(x1, cin ^ 1, &mut and_gates, &mut next_var);
                    let sum = add_and(sum_t1 ^ 1, sum_t2 ^ 1, &mut and_gates, &mut next_var) ^ 1;
                    
                    // CarryOut = Maj(a, b, cin)
                    let ab = add_and(a, b, &mut and_gates, &mut next_var);
                    let bc = add_and(b, cin, &mut and_gates, &mut next_var);
                    let ac = add_and(a, cin, &mut and_gates, &mut next_var);
                    let t = add_and(ab ^ 1, bc ^ 1, &mut and_gates, &mut next_var) ^ 1;
                    let cout = add_and(t ^ 1, ac ^ 1, &mut and_gates, &mut next_var) ^ 1;

                    net_to_lit[node.outputs()[0]] = cout;
                    net_to_lit[node.outputs()[1]] = sum;
                }
            }
        }

        let output_lits: Vec<usize> = self.outputs.iter()
            .map(|nl| get_lit(nl, &net_to_lit))
            .collect();

        AigData {
            inputs: input_lits,
            outputs: output_lits,
            and_gates,
        }
    }
}

impl ToJson for Circuit {
    fn to_json_string(&self) -> String {
        let mut hasher = DefaultHasher::new();
        self.nodes.len().hash(&mut hasher);
        self.nets.len().hash(&mut hasher);
        self.outputs().hash(&mut hasher);
        let graph_id = format!("{:016x}", hasher.finish());

        let payload = json!({
            "graph_id": graph_id,
            "num_nodes": self.nodes.len(),
            "num_nets": self.nets.len(),
            "hypergraph": {
                "nodes": self.nodes.iter().map(|n| {
                    json!({
                        "gate": n.gate().name(),
                        "inputs": n.inputs().iter().map(|lit| (lit.net(), lit.negative() as u8)).collect::<Vec<_>>(),
                        "outputs": n.outputs(),
                    })
                }).collect::<Vec<_>>(),
                "nets": self.nets.iter().map(|n| {
                    json!({
                        "driver": n.driver(),
                        "loads": n.loads(),
                    })
                }).collect::<Vec<_>>(),
            }
        });

        serde_json::to_string(&payload).expect("Serialization failed")
    }
}
