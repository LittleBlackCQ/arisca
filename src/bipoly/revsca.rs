use super::strategy::Strategy;
use super::{Polynomial, VarId};
use super::poly::AlgebraicCircuit;
use crate::circuit::*;

use num_bigint::BigInt;
use num_traits::One;
use std::collections::HashMap;
use log::{debug, warn};

pub struct RevscaStrategy;
impl Strategy for RevscaStrategy {
    fn apply(&self, ac: &AlgebraicCircuit, poly_map: &mut HashMap<NetId, Polynomial>) {
        let mut cncs = self.find_cncs(ac);
        cncs.sort_by_key(|n| ac.var(ac.inner.nodes_at(*n.last().unwrap()).outputs()[0]));

        for cone_nodes in cncs {
            self.compute_cone_poly(ac, &cone_nodes, poly_map);
        }
    }
}

impl RevscaStrategy {
    fn compute_cone_poly(&self, ac: &AlgebraicCircuit, cone_nodes: &[NodeId], poly_map: &mut HashMap<NetId, Polynomial>) {
        let mut internal_nets: Vec<NetId> = cone_nodes.iter()
            .flat_map(|&nid| ac.inner.nodes_at(nid).outputs().iter().cloned())
            .collect();
        
        internal_nets.sort_by_key(|&n| ac.var(n));

        let s_var = ac.var(internal_nets[0]);
        let c_var = ac.var(internal_nets[1]);
        let ha_vars = [s_var, c_var];

        let &root_net = internal_nets.last().unwrap();
        let mut root_poly = poly_map.get(&root_net).unwrap().clone();
        debug!("Root polynomial for converging net {:?}, terms: {:?}\n{:?}", root_net,  root_poly.terms().len(), root_poly);

        for net in internal_nets[2..internal_nets.len()-1].iter().rev() {
            if let Some(gate_poly) = poly_map.get(net) {
                let gate_poly = gate_poly.clone();
                debug!("Polynomial for Net {:?}: {:?}", net, gate_poly);
                if let Some(mono) = gate_poly.leading_term() {
                    if mono.degree() != 1 || !mono.coeff().is_one() {
                        warn!("Net {:?} polynomial has non-unit leading term: {:?}", net, mono);
                    }
                    
                    let factor = root_poly.divide_by_term(mono.term());
                    
                    if !factor.is_zero() {
                        root_poly -= gate_poly * factor;
                    }
                }
            }
            debug!("Terms after reducing: {:?}", root_poly.terms().len());
            self.clean_vanishing_monomials(&mut root_poly, &ha_vars);
            debug!("Terms after removing x{:?}*x{:?}: {:?}", ha_vars[0], ha_vars[1], root_poly.terms().len());
        }
        debug!("Root polynomial for converging net {:?} after removing, terms: {:?}\n{:?}", root_net,  root_poly.terms().len(), root_poly);
        poly_map.insert(root_net, root_poly);
    }

    fn clean_vanishing_monomials(&self, poly: &mut Polynomial, term: &[VarId; 2]) {
        let factor = poly.divide_by_term(term);
        if !factor.is_zero() {
            poly.sub_assign(&(factor * Polynomial::term(term, BigInt::one())));
        }
    }

    fn find_cncs(&self, ac: &AlgebraicCircuit) -> Vec<Vec<NodeId>> {
        let mut cncs = Vec::new();
        let not_arithmetic = |n: NodeId| !matches!(ac.inner.nodes_at(n).gate(), Gate::HalfAdder | Gate::FullAdder);
        
        for (ha_node_id, node) in ac.inner.nodes().iter().enumerate() {
            if matches!(node.gate(), Gate::HalfAdder) {
                let outs = node.outputs();
                let (sum_net, carry_net) = (outs[0], outs[1]);

                let sum_paths = self.find_paths(ac, sum_net, &not_arithmetic);
                let carry_paths = self.find_paths(ac, carry_net, &not_arithmetic);

                for sp in &sum_paths {
                    for cp in &carry_paths {
                        if let Some(meet_idx_s) = sp.iter().position(|n| cp.contains(n)) {
                            let meet_node = sp[meet_idx_s];
                            if meet_node == ha_node_id { continue; }

                            let meet_idx_c = cp.iter().position(|&n| n == meet_node).unwrap();
                            
                            let mut cone = vec![ha_node_id];
                            cone.extend(sp.iter().take(meet_idx_s));
                            cone.extend(cp.iter().take(meet_idx_c));
                            cone.push(meet_node);

                            cncs.push(cone);
                            debug!("Find converging node {:?} for half adder {:?}", meet_node, ha_node_id);
                        }
                    }
                }
            }
        }
        cncs
    }

    fn find_paths<F>(&self, ac: &AlgebraicCircuit, start_net: NetId, pred: &F) -> Vec<Vec<NodeId>> 
    where F: Fn(NodeId) -> bool {
        let mut paths = Vec::new();
        let mut stack: Vec<Vec<NodeId>> = Vec::new();

        for &load_id in ac.inner.nets_at(start_net).loads() {
            if pred(load_id) {
                stack.push(vec![load_id]);
            }
        }

        while let Some(path) = stack.pop() {
            let &curr = path.last().unwrap();
            let mut extended = false;
            for &out_net in ac.inner.nodes_at(curr).outputs() {
                for &load_id in ac.inner.nets_at(out_net).loads() {
                    if !path.contains(&load_id) && pred(load_id) {
                        let mut new_path = path.clone();
                        new_path.push(load_id);
                        stack.push(new_path);
                        extended = true;
                    }
                }
            }

            if !extended {
                paths.push(path);
            }
        }
        paths
    }
}