use super::{VarId, Polynomial, PolyVerifier, AlgebraicCircuit};
use super::strategy::{Strategy, ReductionAction};
use crate::circuit::*;

use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub struct RevscaStrategy {
    cones: Vec<Cone>,
    adj: HashMap<VarId, Vec<VarId>>,
    in_degree: HashMap<VarId, usize>,
    queue: Vec<VarId>,
}

impl Strategy for RevscaStrategy {
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
        for cone in self.cones.iter() {
            if cone.nets.len() < 3 { continue; }
            let is_converging = cone.inputs.iter().filter_map(|&net_id| {
                if let Some(driver) = ac.circuit.nets_at(net_id).driver() {
                    let node = ac.circuit.nodes_at(driver);
                    if *node.gate() == Gate::HalfAdder {
                        Some(node)
                    } else { None }
                } else { None }
            }).any(|node| {
                let outputs = node.outputs();
                outputs.iter().all(|net_id| { cone.inputs.contains(net_id) })
            });
            
            let mut var_iter = cone.nets[..cone.nets.len()-1]
                .iter()
                .rev()
                .map(|net_id| ac.var(*net_id))
                .collect::<Vec<VarId>>()
                .into_iter();

            let root_var = ac.var(cone.root);
            let post_reduce_fn: fn(&mut Polynomial) = if is_converging { Self::post_reduce } else { |_poly| {} } ;
            if let Some(target_poly) = poly_map.get(&root_var).cloned() {
                let result_poly = PolyVerifier::poly_reduce(
                    target_poly,
                    poly_map,
                    |_current_poly| {
                        var_iter.next()
                                .map(ReductionAction::Reduce)
                                .unwrap_or(ReductionAction::Stop)
                    },
                    post_reduce_fn
                );
                poly_map.insert(root_var, result_poly);
            }
        }
    }

    fn init_order(&mut self, ac: &AlgebraicCircuit) {
        self.adj.clear();
        self.in_degree.clear();

        for outlit in ac.circuit.outputs() {
            self.in_degree.entry(ac.var(outlit.net())).or_insert(0);
        }

        for node in ac.circuit.nodes() {
            if node.outputs().len() != 2 { continue; }
            for out in node.outputs() {
                for inp in node.inputs() {
                    *self.in_degree.entry(ac.var(inp.net())).or_insert(0) += 1;
                    self.adj.entry(ac.var(*out)).or_insert(Vec::new()).push(ac.var(inp.net()));
                }
            }
            *self.in_degree.entry(ac.var(node.outputs()[0])).or_insert(0) += 1;
            self.adj.entry(ac.var(node.outputs()[1])).or_insert(Vec::new()).push(ac.var(node.outputs()[0]));
        }
        for cone in self.cones.iter() {
            for &inp in cone.inputs.iter() {
                *self.in_degree.entry(ac.var(inp)).or_insert(0) += 1;
                self.adj.entry(ac.var(cone.root)).or_insert(Vec::new()).push(ac.var(inp));
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

        let selection = if self.queue.len() == 1 {
            (self.queue[0], None) 
        } else {
            self.lazy_suite_var(current_poly, poly_map)
        };

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
                Self::post_reduce
            );

            let rate = (result_poly.size() as f64 - current_size) / current_size;
            
            let selection = (var, Some(result_poly));
            if rate < threshold {
                return selection;
            }
            match best_candidate {
                None => best_candidate = Some((rate, selection)),
                Some((best_rate, _)) if rate > best_rate => best_candidate = Some((rate, selection)),
                _ => {}
            }
        }
        best_candidate.map(|(_, selection)| selection).expect("Queue should not be empty")
    }
}