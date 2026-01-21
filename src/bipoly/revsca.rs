use super::{VarId, Polynomial, PolyVerifier, AlgebraicCircuit};
use super::strategy::{Strategy, ReductionAction};
use crate::circuit::*;

use num_bigint::BigInt;
use itertools::Itertools;
use std::collections::{HashMap, HashSet};
use log::debug;

#[derive(Default)]
pub struct RevscaStrategy {
    cones: Vec<Cone>,
    is_convergings : Vec<bool>,
    adj: HashMap<VarId, Vec<VarId>>,
    in_degree: HashMap<VarId, usize>,
    queue: Vec<VarId>,
    modulus: Option<BigInt>
}

impl Strategy for RevscaStrategy {
    fn init(&mut self, ac: &AlgebraicCircuit) {
        self.modulus = ac.modulus.clone();
    }

    fn gen_var_map(&self, circuit: &Circuit) -> Vec<VarId> {
        let mut net_to_var: Vec<i32> = (0..circuit.nets().len() as i32).collect();
        circuit.nodes().iter().filter(|&node| {
            *node.gate() == Gate::HalfAdder 
        }).enumerate().for_each(|(i, node)| {
            for (j, net_id) in node.outputs().iter().enumerate() {
                net_to_var[*net_id] = -((3 * i + j + 1) as i32)
            }
        });
        net_to_var
    }

    fn pre_reduce(&mut self, ac: &AlgebraicCircuit, poly_map: &mut HashMap<VarId, Polynomial>) {
        if self.cones.is_empty() {
            self.find_fanoutfree_cones(ac.circuit);
        }
        if self.is_convergings.is_empty() {
            self.detect_converging_cones(ac);
        }

        for (cone_idx, cone) in self.cones.iter().enumerate() {
            let is_converging = self.is_convergings[cone_idx];
            debug!("Reduce: {:?}, is converging: {:?}", cone, is_converging);

            let mut var_iter = cone.nets[..cone.nets.len()-1]
                .iter()
                .rev()
                .map(|net_id| ac.var(*net_id))
                .collect::<Vec<VarId>>()
                .into_iter();

            let root_var = ac.var(cone.root);
            if let Some(target_poly) = poly_map.get(&root_var).cloned() {
                let result_poly = PolyVerifier::poly_reduce(
                    target_poly,
                    poly_map,
                    move |_current_poly| {
                        var_iter.next()
                            .map(ReductionAction::Reduce)
                            .unwrap_or(ReductionAction::Stop)
                    },
                    |current_poly| {
                        if is_converging {
                            Self::post_reduce(current_poly);
                        }
                    }
                );
                poly_map.insert(root_var, result_poly);
            }
        }
    }

    fn init_order(&mut self, ac: &AlgebraicCircuit) {
        self.adj.clear();
        self.in_degree.clear();

        // let carry_to_sum: HashMap<_, _> = ac.circuit.nodes().iter().filter_map(|node| {
        //     if node.outputs().len() == 2 {
        //         Some((node.outputs()[0], ac.var(node.outputs()[1])))
        //     } else { None }
        // }).collect();

        for outlit in ac.circuit.outputs() {
            self.in_degree.entry(ac.var(outlit.net())).or_insert(0);
        }
        for cone in self.cones.iter() {
            self.in_degree.entry(ac.var(cone.root)).or_insert(0);
        }

        for node in ac.circuit.nodes() {
            if node.outputs().len() != 2 { continue; }

            let node_out_vars: Vec<_> = node.outputs().iter().map(|&n| ac.var(n)).collect();
            for inp in node.inputs() {
                let inp_var = ac.var(inp.net());
                for &out_var in node_out_vars.iter() {
                    *self.in_degree.entry(inp_var).or_insert(0) += 1;
                    self.adj.entry(out_var).or_default().push(inp_var);

                    // if let Some(&partner_sum_var) = carry_to_sum.get(&inp.net()) {
                    //     *self.in_degree.entry(partner_sum_var).or_insert(0) += 1;
                    //     self.adj.entry(out_var).or_default().push(partner_sum_var);
                    // }
                }
            }
            let (carry_var, sum_var) = (ac.var(node.outputs()[0]), ac.var(node.outputs()[1]));
            *self.in_degree.entry(carry_var).or_insert(0) += 1;
            self.adj.entry(sum_var).or_default().push(carry_var);
        }
        for cone in self.cones.iter() {
            let root_var = ac.var(cone.root);
            for &inp in cone.inputs.iter() {
                let inp_var = ac.var(inp);
                *self.in_degree.entry(inp_var).or_insert(0) += 1;
                self.adj.entry(root_var).or_default().push(inp_var);

                // if let Some(&partner_sum_var) = carry_to_sum.get(&inp) {
                //      *self.in_degree.entry(partner_sum_var).or_insert(0) += 1;
                //      self.adj.entry(root_var).or_default().push(partner_sum_var);
                // }
            }
        }
        self.queue = self.in_degree.iter().filter_map(|(&v, &degree)| { if degree == 0 { Some(v) } else { None } }).collect();
    }

    fn next_reduction_var(
        &mut self, 
        current_poly: &Polynomial, 
        poly_map: &HashMap<VarId, Polynomial>
    ) -> ReductionAction { 
        if self.queue.is_empty() || current_poly.is_zero() {
            return ReductionAction::Stop;
        }
        // count occurrences and sort (very important)
        let mut occ_counts = HashMap::new();
        for term in current_poly.terms() {
            for var in term.term() {
                *occ_counts.entry(var).or_insert(0) += 1;
            }
        }
        self.queue.sort_by_key(|&v| {
            occ_counts.get(&v).unwrap_or(&0);
        });
        debug!("Current queue: {:?}", self.queue);

        let selection = if self.queue.len() == 1 {
            (self.queue[0], None) 
        } else {
            self.lazy_suite_var(current_poly, poly_map)
        };
        debug!("Choose Var {:?}", selection.0);

        self.remove_and_update_graph(selection.0);
        if let Some(poly) = selection.1 {
            ReductionAction::Replace(poly, selection.0)
        } else {
            ReductionAction::Reduce(selection.0)
        }
    }

    fn post_reduce(poly: &mut Polynomial) {
        poly.remove_mono_by(|m| {
            for pair in m.term().windows(2) {
                let (var_i, var_j) = (pair[0], pair[1]);
                if var_j >= 0 {
                    return false;
                } else if (var_j - var_i) == 1 {
                    return true;
                } 
            }
            false
        });
    }
}

impl RevscaStrategy {
    fn find_fanoutfree_cones(&mut self, circuit: &Circuit) {
        let terminals: HashSet<NetId> = circuit.nets().iter().enumerate().filter_map(|(net_id, net)| {
            if let Some(driver) = net.driver() {
                if circuit.nodes_at(driver).is_multioutput() {
                    None
                } else if net.loads().len() > 1 || net.loads().is_empty() || circuit.nodes_at(net.loads()[0]).is_multioutput(){
                    Some(net_id)
                } else {
                    None
                }
            } else {
                None
            }
        }).collect();
        
        self.cones = terminals.iter().map(|&root| {
            circuit.get_levelized_cone(root, |&net_id| {
                let net = circuit.nets_at(net_id);
                if net_id != root && terminals.contains(&net_id) {
                    true
                } else if let Some(driver) = net.driver() {
                    circuit.nodes_at(driver).is_multioutput()
                } else {
                    true
                }
            })
        }).collect()
    }

    fn detect_converging_cones(&mut self, ac: &AlgebraicCircuit) { 
        let root_to_cone: HashMap<_, _> = self.cones.iter()
            .map(|c| (c.root, c))
            .collect();

        let mut updates = Vec::new();

        for (cone_idx, cone) in self.cones.iter().enumerate() {
            if cone.nodes.len() < 3 { continue; };

            fn expand_recursive(
                current_cone: &Cone,
                ac: &AlgebraicCircuit,
                root_to_cone: &HashMap<NetId, &Cone>,
                acc_nets: &mut Vec<NetId>,
                acc_nodes: &mut Vec<NodeId>,
                acc_inputs: &mut HashSet<NetId>,
            ) {
                let current_input_set: HashSet<&NetId> = current_cone.inputs.iter().collect();

                for &node_id in &current_cone.nodes {
                    let node = ac.circuit.nodes_at(node_id);
                    
                    for input_lit in node.inputs() {
                        let input_net = input_lit.net();

                        if current_input_set.contains(&input_net) {
                            let should_expand = root_to_cone
                                .get(&input_net)
                                .filter(|upstream| upstream.nodes.len() < 5);

                            if let Some(upstream_cone) = should_expand {
                                
                                expand_recursive(
                                    upstream_cone,
                                    ac,
                                    root_to_cone,
                                    acc_nets,
                                    acc_nodes,
                                    acc_inputs,
                                );
                            } else {
                                acc_inputs.insert(input_net);
                            }
                        } 
                    }

                    acc_nodes.push(node_id);
                    acc_nets.push(node.outputs()[0]);
                }
            }
            
            let mut final_nets = Vec::new();
            let mut final_nodes = Vec::new();
            let mut final_inputs = HashSet::new();


            expand_recursive(
                cone,
                ac,
                &root_to_cone,
                &mut final_nets,
                &mut final_nodes,
                &mut final_inputs,
            );

            let is_converging = final_inputs
                .iter()
                .map(|&input| ac.var(input))
                .sorted()
                .tuple_windows()
                .any(|(a, b)| b < 0 && (b - a) == 1);

            if is_converging {
                updates.push((cone_idx, final_inputs, final_nets, final_nodes));
            }
        }
        self.is_convergings.resize(self.cones.len(), false);
        for (cone_idx, new_inputs, new_nets, new_nodes) in updates {
            self.is_convergings[cone_idx] = true;
            let cone = &mut self.cones[cone_idx];
            cone.inputs = new_inputs.iter().copied().collect();
            cone.nets = new_nets;
            cone.nodes = new_nodes;
        }
    }

    fn remove_and_update_graph(&mut self, target_var: VarId) {
        if let Some(pos) = self.queue.iter().position(|&v| v == target_var) {
            self.queue.remove(pos);
        }
        if let Some(neighbors) = self.adj.get(&target_var) {
            for &neighbor in neighbors {
                if let Some(degree) = self.in_degree.get_mut(&neighbor) {
                    *degree -= 1;
                    if *degree == 0 {
                        self.queue.push(neighbor);
                    }
                }
            }
        }
    }

    fn lazy_suite_var(&mut self, current_poly: &Polynomial, poly_map: &HashMap<VarId, Polynomial>) -> (VarId, Option<Polynomial>) {
        let threshold = 0.1;
        let current_size = current_poly.size() as f64;

        let mut best_candidate: Option<(f64, (VarId, Option<Polynomial>))> = None;
        for &var in self.queue.iter() {
            let mut one_shot_iter = std::iter::once(var);
            let result_poly = PolyVerifier::poly_reduce(
                current_poly.clone(),
                poly_map,
                |_current_poly| {
                    one_shot_iter.next().map(ReductionAction::Reduce).unwrap_or(ReductionAction::Stop)
                },
                |current_poly| {
                    Self::post_reduce(current_poly);
                    if let Some(m) = &self.modulus {
                        current_poly.mod_by_const(m);
                    }
                }
            );

            let rate = (result_poly.size() as f64 - current_size) / current_size;
            
            let selection = (var, Some(result_poly));
            if rate < threshold {
                return selection;
            }
            match best_candidate {
                None => best_candidate = Some((rate, selection)),
                Some((best_rate, _)) if rate < best_rate => best_candidate = Some((rate, selection)),
                _ => {}
            }
        }
        best_candidate.map(|(_, selection)| selection).expect("Queue should not be empty")
    }
}