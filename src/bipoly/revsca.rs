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
    penalty: HashMap<VarId, u32>,
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
        self.sort_queue_by_occ_penalty(current_poly, poly_map);
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
            circuit.get_dfs_cone(root, |&net_id| {
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

            let mut final_nodes = Vec::new();
            let mut final_nets = Vec::new();
            let mut final_inputs = HashSet::new();

            let mut queue = vec![cone];

            while let Some(curr_cone) = queue.pop() { 
                for &input_net in curr_cone.inputs.iter() {
                    let should_expand = root_to_cone.get(&input_net)
                        .filter(|upstream| {
                            upstream.nodes.len() < 5
                        });
                    if let Some(&upstream_cone) = should_expand {
                        queue.push(upstream_cone);
                    } else {
                        final_inputs.insert(input_net);
                    }
                }
            }

            let is_converging = final_inputs
                .iter()
                .map(|&input| ac.var(input))
                .sorted()
                .tuple_windows()
                .filter(|&(a, b)| b < 0 && (b - a) == 1)
                .count() * 8 > final_inputs.len() * 3; // has is more than 3/4 of the inputs

            if is_converging {
                fn collect_ordered(net: NetId, ac: &AlgebraicCircuit, inputs: &HashSet<NetId>, acc_nets: &mut Vec<NetId>, acc_nodes: &mut Vec<NodeId>) {
                    if inputs.contains(&net) { return; }
                    
                    if let Some(driver) = ac.circuit.nets_at(net).driver() {
                        let node = ac.circuit.nodes_at(driver);
                        
                        let children: Vec<NetId> = node.inputs().iter().map(|l| l.net()).sorted().rev().collect();

                        for child in children {
                            collect_ordered(child, ac, inputs, acc_nets, acc_nodes);
                        }

                        acc_nodes.push(driver);
                        acc_nets.push(net);
                    }
                }
                collect_ordered(cone.root, ac, &final_inputs, &mut final_nets, &mut final_nodes);
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
            } else {
                *self.penalty.entry(var).or_insert(1) *= 2;
            }
            match best_candidate {
                None => best_candidate = Some((rate, selection)),
                Some((best_rate, _)) if rate < best_rate => best_candidate = Some((rate, selection)),
                _ => {}
            }
        }
        best_candidate.map(|(_, selection)| selection).expect("Queue should not be empty")
    }

    fn sort_queue_by_occ_penalty(&mut self, current_poly: &Polynomial, poly_map: &HashMap<VarId, Polynomial>) {
        if self.queue.is_empty() || self.queue.len() == 1 {
            return;
        }
        let mut stats: Vec<(_, u32)> = self.queue.iter().map(|&v| (v, 0)).collect();
        stats.sort_by_key(|(v, _)| *v);

        let min_q = stats.first().unwrap().0;
        let max_q = stats.last().unwrap().0;

        for mono in current_poly.terms() {
            let vars = mono.term();

            if vars.is_empty() || *vars.last().unwrap() < min_q {
                continue;
            }
            if vars[0] > max_q {
                break;
            }

            let (mut i, mut j) = (0, 0);
            while i < stats.len() && j < vars.len() {
                let target = stats[i].0;
                let current = vars[j];

                if target == current {
                    stats[i].1 += 1;
                    i += 1;
                    j += 1;
                } else if current < target {
                    j += 1;
                } else {
                    i += 1;
                }
            }
        }

        self.queue.sort_by_key(|v| {
            stats.binary_search_by_key(v, |(var, _)| *var)
                 .map(|idx| stats[idx].1)
                 .unwrap_or(0) * 
            *self.penalty.entry(*v).or_insert(1) *
            poly_map.get(v).unwrap_or(&Polynomial::zero()).size() as u32
        });
    }
}
