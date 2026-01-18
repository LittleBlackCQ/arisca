use super::*;
use super::sim::Simulator;
use std::collections::{BTreeSet, HashMap, HashSet};

use log::info;

pub trait ExtractorStrategy {
    fn cut_size(&self) -> usize;
    fn target_gate(&self) -> Gate;
    fn match_tt(&self, tt: &[bool]) -> Option<(bool, Vec<bool>)>;
}

#[derive(Debug, Clone)]
pub struct Match {
    pub cone: Cone,
    pub output_negated: bool,
    pub input_negations: Vec<bool>,
    pub gate: Gate,
}

pub struct GenericExtractor;
impl GenericExtractor {
    pub fn try_match(
        circuit: &Circuit, 
        root: NetId, 
        cut: &BTreeSet<NetId>, 
        strategy: &dyn ExtractorStrategy,
        sim: &mut Simulator,
    ) -> Option<Match> {
        if cut.len() != strategy.cut_size() { return None; }

        let cone= circuit.get_levelized_cone(root, |net| cut.contains(&net));

        let tt: Vec<bool> = sim.get_partial_tt(&cone.inputs, &cone.nets, &[root]).iter().map(|r| r[0]).collect();
      
        strategy.match_tt(&tt).map(|(output_negated, input_negations)| Match { 
            cone, output_negated, input_negations, gate: strategy.target_gate(),
        })
    }

    pub fn run<S: ExtractorStrategy>(circuit: &Circuit, strategy: S) -> Circuit {
        let mut sim = Simulator::new(circuit);

        let cut_db = circuit.get_cuts(strategy.cut_size(), 20);
        let mut matches = Vec::new();

        for root in 0..circuit.nets().len() {
            for cut in cut_db[root].iter() {
                if let Some(m) = Self::try_match(circuit, root, cut, &strategy, &mut sim) {
                    matches.push(m);
                    break;
                }
            }
        }
        Self::rebuild_circuit(circuit, matches)
    }

    pub fn rebuild_circuit(circuit: &Circuit, matches: Vec<Match>) -> Circuit {
        let negated_outputs: HashSet<NetId> = matches.iter()
            .filter_map(|m| if m.output_negated { Some(m.cone.root) } else { None })
            .collect();

        let match_lookup: HashMap<NetId, &Match> = matches.iter().map(|m| (m.cone.root, m)).collect();
        // recursive reconstruction - build when needed
        fn reconstruct_node(
            circuit: &Circuit, 
            matches: &[Match], 
            negated_outputs: &HashSet<NetId>, 
            net_id: &NetId, 
            match_lookup: &HashMap<NetId, &Match>,
            new_circuit: &mut Circuit, 
            net_map: &mut HashMap<NetId, NetId>
        ) -> NetId {
            if let Some(&id) = net_map.get(net_id) { return id; }

            // mapped node
            if let Some(m) = match_lookup.get(net_id) {
                let inputs: Vec<NetLit> = m.cone.inputs.iter().zip(m.input_negations.iter())
                    .map(|(n, neg)| {
                        let mapped = reconstruct_node(circuit, matches, negated_outputs, n, match_lookup,  new_circuit, net_map);
                        NetLit::new(mapped, neg ^ negated_outputs.contains(n))
                    })
                    .collect();
                let out = new_circuit.add_gate(m.gate.clone(), inputs)[0];
                net_map.insert(m.cone.root, out);
                out
            } else { // old node
                let node = circuit.nodes_at(circuit.nets_at(*net_id).driver().expect("Floating net!"));
                let inputs: Vec<NetLit> = node.inputs().iter().map(|lit| {
                        let mapped = reconstruct_node(circuit, matches, negated_outputs, &lit.net(), match_lookup, new_circuit, net_map);
                        NetLit::new(mapped, lit.negative() ^ negated_outputs.contains(&lit.net()))
                    })
                    .collect();
                let out = new_circuit.add_gate(node.gate().clone(), inputs)[0];
                net_map.insert(*net_id, out);
                out
            }
        }

        let mut new_circuit = Circuit::empty();
        let mut net_map = HashMap::new();
        net_map.insert(0, 0);

        // PIs
        for net_id in circuit.inputs() {
            let new_id = new_circuit.add_input();
            net_map.insert(*net_id, new_id);
        }
        
        // POs
        for out_lit in circuit.outputs() {
            let mapped = reconstruct_node(circuit, matches.as_slice(), &negated_outputs, &out_lit.net(), &match_lookup, &mut new_circuit, &mut net_map);
            new_circuit.set_output(mapped, out_lit.negative() ^ negated_outputs.contains(&out_lit.net()));
        }

        new_circuit
    }
}

// --- Specific Strategies ---

pub struct XorExtractor;
impl ExtractorStrategy for XorExtractor {
    fn cut_size(&self) -> usize { 2 }
    fn target_gate(&self) -> Gate { Gate::Xor }
    fn match_tt(&self, tt: &[bool]) -> Option<(bool, Vec<bool>)> {
        match tt {
            [false, true, true, false] => Some((false, vec![false; 2])),
            [true, false, false, true] => Some((true, vec![false; 2])),
            _ => None
        }
    }
}

pub struct AndExtractor;
impl ExtractorStrategy for AndExtractor {
    fn cut_size(&self) -> usize { 2 }
    fn target_gate(&self) -> Gate { Gate::And }
    fn match_tt(&self, tt: &[bool]) -> Option<(bool, Vec<bool>)> {
        for mask in 0..4 {
            let negs: Vec<bool> = (0..2).map(|i| (mask >> i) & 1 == 1).collect();
            let exp_tt: Vec<bool> = (0..4).map(|i| {
                let v0 = ((i & 1) == 1) ^ negs[0];
                let v1 = ((i >> 1) & 1 == 1) ^ negs[1];
                v0 & v1
            }).collect();

            if tt == &exp_tt { return Some((false, negs)); }
            if tt.iter().zip(&exp_tt).all(|(a, b)| *a == !*b) { return Some((true, negs)); }
        }
        None
    }
}

pub struct Xor3Extractor;
impl ExtractorStrategy for Xor3Extractor {
    fn cut_size(&self) -> usize { 3 }
    fn target_gate(&self) -> Gate { Gate::Xor3 }
    fn match_tt(&self, tt: &[bool]) -> Option<(bool, Vec<bool>)> {
        match tt {
            [false, true, true, false, true, false, false, true] => Some((false, vec![false; 3])),
            [true, false, false, true, false, true, true, false] => Some((true, vec![false; 3])),
            _ => None
        }
    }
}

pub struct MajExtractor;
impl ExtractorStrategy for MajExtractor {
    fn cut_size(&self) -> usize { 3 }
    fn target_gate(&self) -> Gate { Gate::Maj }
    fn match_tt(&self, tt: &[bool]) -> Option<(bool, Vec<bool>)> {
        for mask in 0..8 {
            let negs: Vec<bool> = (0..3).map(|i| (mask >> i) & 1 == 1).collect();
            let exp_tt: Vec<bool> = (0..8).map(|i| {
                let cnt = (0..3).filter(|b| ((i >> b) & 1 == 1) ^ negs[*b]).count();
                cnt >= 2
            }).collect();

            if tt == &exp_tt { return Some((false, negs)); }
            if tt.iter().zip(&exp_tt).all(|(a, b)| *a == !*b) { return Some((true, negs)); }
        }
        None
    }
}

// --- Adder Extractor (Multi-output) ---
#[derive(Debug, Clone)]
struct AdderMatch {
    sum: Match,
    carry: Match,
    gate: Gate,
}

pub struct AdderExtractor;
impl AdderExtractor {
    pub fn run(circuit: &Circuit) -> Circuit {
        let mut sim = Simulator::new(circuit);

        let cut_db = circuit.get_cuts(3, 20);
        let mut fa_candidates: HashMap<BTreeSet<NetId>, Vec<Match>> = HashMap::new();
        let mut ha_candidates: HashMap<BTreeSet<NetId>, Vec<Match>> = HashMap::new();
        let fa_strategies: [&dyn ExtractorStrategy; 2] = [&Xor3Extractor, &MajExtractor];
        let ha_strategies: [&dyn ExtractorStrategy; 2] = [&XorExtractor, &AndExtractor];

        for root in 0..circuit.nets().len() {
            for cut in &cut_db[root] {
                if cut.len() == 2 {
                    for strategy in ha_strategies {
                        if let Some(m) = GenericExtractor::try_match(circuit, root, cut, strategy, &mut sim) {
                            ha_candidates.entry(cut.clone()).or_default().push(m);
                            break;
                        }
                    }
                } else if cut.len() == 3 {
                    for strategy in fa_strategies {
                        if let Some(m) = GenericExtractor::try_match(circuit, root, cut, strategy, &mut sim) {
                            fa_candidates.entry(cut.clone()).or_default().push(m);
                            break;
                        }
                    }
                }
            }
        }

        let mut matches = Vec::new();
        let mut consumed_roots = HashSet::new();

        let mut find_adder = |sums: &[Match], carries: &[Match], gate_type: Gate| {
            for (s, c) in sums.iter().flat_map(|s| carries.iter().map(move |c| (s, c))) {
                if consumed_roots.contains(&s.cone.root) || consumed_roots.contains(&c.cone.root) { continue; }

                let all_cone_nodes: HashSet<NodeId> = s.cone.nodes.iter().chain(c.cone.nodes.iter()).cloned().collect(); 
                let all_cone_nets: HashSet<NetId> = s.cone.nets.iter().chain(c.cone.nets.iter()).cloned().collect();

                if !all_cone_nets.iter().all(|&net| {
                    let is_output = circuit.outputs().iter().any(|o| o.net() == net);
                    if net == s.cone.root || net == c.cone.root {
                        is_output || circuit.nets_at(net).loads().iter()
                            .any(|l| !all_cone_nodes.contains(l))
                    } else {
                        !is_output && circuit.nets_at(net).loads().iter()
                            .all(|l| all_cone_nodes.contains(l))
                    }
                }) { continue; }

                matches.push(AdderMatch { sum: (*s).clone(), carry: (*c).clone(), gate: gate_type });

                consumed_roots.insert(s.cone.root);
                consumed_roots.insert(c.cone.root);
            }
        };

        // full adder first
        for ms in fa_candidates.values() {
            let mut xor3s = Vec::new();
            let mut majs = Vec::new();
            ms.iter().for_each(|m| if m.gate == Gate::Xor3 { xor3s.push(m.clone()) } else if m.gate == Gate::Maj { majs.push(m.clone()) });
            find_adder(&xor3s, &majs, Gate::FullAdder);
        }
        for ms in ha_candidates.values() {
            let mut xors = Vec::new();
            let mut ands = Vec::new();
            ms.iter().for_each(|m| if m.gate == Gate::Xor { xors.push(m.clone()) } else if m.gate == Gate::And { ands.push(m.clone()) });
            find_adder(&xors, &ands, Gate::HalfAdder);
        }

        Self::rebuild_adders(circuit, matches)
    }

    fn rebuild_adders(circuit: &Circuit, matches: Vec<AdderMatch>) -> Circuit {
        // Phase 1: Prepare negated outputs
        let mut match_lookup = HashMap::new();
        let mut negated_outputs = HashSet::new();

        for m in matches.iter() {
            match_lookup.insert(m.sum.cone.root, m);
            match_lookup.insert(m.carry.cone.root, m);

            if m.carry.output_negated {negated_outputs.insert(m.carry.cone.root);}
            let input_parity_diff = m.sum.input_negations.iter()
                .zip(m.carry.input_negations.iter())
                .filter(|(a, b)| a != b)
                .count() % 2 != 0;
            
            if m.sum.output_negated ^ input_parity_diff {
                negated_outputs.insert(m.sum.cone.root);
            }
        }

        // Phase 2: Recursive Reconstruction
        fn reconstruct(
            circuit: &Circuit, 
            matches: &[AdderMatch], 
            negated_outputs: &HashSet<NetId>, 
            net_id: &NetId, 
            match_lookup: &HashMap<NetId, &AdderMatch>,
            new_circuit: &mut Circuit, 
            net_map: &mut HashMap<NetId, NetId>
        ) -> NetId {
            if let Some(&id) = net_map.get(net_id) { return id; }

            // mapped node
            if let Some(m) = match_lookup.get(net_id) {
                let inputs: Vec<NetLit> = m.carry.cone.inputs.iter().zip(&m.carry.input_negations).map(|(n, neg)| {
                    let mapped = reconstruct(circuit, matches, negated_outputs, n, match_lookup, new_circuit, net_map);
                    NetLit::new(mapped,  neg ^ negated_outputs.contains(n))
                }).collect();

                let outs = new_circuit.add_gate(m.gate.clone(), inputs);
                net_map.insert(m.carry.cone.root, outs[0]);
                net_map.insert(m.sum.cone.root, outs[1]);

                net_map[net_id]
            } else { // old node
                let node = circuit.nodes_at(circuit.nets_at(*net_id).driver().expect("Floating net!"));
                let inputs: Vec<NetLit> = node.inputs().iter().map(|lit| {
                    let mapped = reconstruct(circuit, matches, negated_outputs, &lit.net(), match_lookup, new_circuit, net_map);
                    NetLit::new(mapped, lit.negative() ^ negated_outputs.contains(&lit.net()))
                }).collect();

                let out = new_circuit.add_gate(node.gate().clone(), inputs)[0];
                net_map.insert(*net_id, out);
                out
            }
        }

        let mut new_circuit = Circuit::empty();
        let mut net_map = HashMap::new();
        net_map.insert(0, 0);

        // PIs
        for net_id in circuit.inputs() {
            net_map.insert(*net_id, new_circuit.add_input());
        }
        
        // POs
        for out_lit in circuit.outputs() {
            let mapped = reconstruct(circuit, &matches, &negated_outputs, &out_lit.net(), &match_lookup, &mut new_circuit, &mut net_map);
            new_circuit.set_output(mapped, out_lit.negative() ^ negated_outputs.contains(&out_lit.net()));
        }

        info!("Mapped FA = {:?} Mapped HA = {:?}", new_circuit.nodes().iter().filter(|n| { *n.gate() == Gate::FullAdder }).count(), new_circuit.nodes().iter().filter(|n| { *n.gate() == Gate::HalfAdder }).count());
        new_circuit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_xor_extraction() {
        let mut circuit = Circuit::empty();
        
        let in_0 = circuit.add_input();
        let in_1 = circuit.add_input();

        // Construct: (in_0 & !in_1)
        let t0_inputs = vec![
            NetLit::new(in_0, false),
            NetLit::new(in_1, true),
        ];
        let t0 = circuit.add_gate(Gate::And, t0_inputs)[0];

        // Construct: (!in_0 & in_1)
        let t1_inputs = vec![
            NetLit::new(in_0, true),
            NetLit::new(in_1, false),
        ];
        let t1 = circuit.add_gate(Gate::And, t1_inputs)[0];

        // Construct: t0 | t1
        let out_inputs = vec![
            NetLit::new(t0, false),
            NetLit::new(t1, false),
        ];
        let out = circuit.add_gate(Gate::Or, out_inputs)[0];

        circuit.set_output(out, false);

        // Run extractor
        let new_circuit = GenericExtractor::run(&circuit, XorExtractor);

        // Assertions
        assert_eq!(new_circuit.inputs().len(), 2);
        assert_eq!(new_circuit.outputs().len(), 1);

        // verify the single gate is XOR (assuming Gate::Xor)
        let out_net = new_circuit.outputs()[0].net();
        let driver_node = new_circuit.nets_at(out_net).driver().unwrap();
        assert_eq!(*new_circuit.nodes_at(driver_node).gate(), Gate::Xor);
        assert_eq!(new_circuit.nodes().len(), 1);
        assert_eq!(Simulator::compute_tt(&circuit), Simulator::compute_tt(&new_circuit));
    }

    #[test]
    fn test_shared_logic_xor3_extraction() {
        let mut circuit = Circuit::empty();

        // Inputs: a, b, c, d
        let a = circuit.add_input();
        let b = circuit.add_input();
        let c = circuit.add_input();
        let d = circuit.add_input();

        // Intermediate: e = XOR(b, c)
        // Using simple Gate::Xor2 for setup
        let e_inputs = vec![NetLit::new(b, false), NetLit::new(c, false)];
        let e = circuit.add_gate(Gate::Xor, e_inputs)[0];

        // Output 1: XOR(a, e) -> XOR(a, b, c)
        let out1_inputs = vec![NetLit::new(a, false), NetLit::new(e, false)];
        let out1 = circuit.add_gate(Gate::Xor, out1_inputs)[0];

        // Output 2: XOR(d, e) -> XOR(d, b, c)
        let out2_inputs = vec![NetLit::new(d, false), NetLit::new(e, false)];
        let out2 = circuit.add_gate(Gate::Xor, out2_inputs)[0];

        circuit.set_output(out1, false);
        circuit.set_output(out2, false);

        let new_circuit = GenericExtractor::run(&circuit, Xor3Extractor);
        
        // Check Output 1
        let new_out1_lit = new_circuit.outputs()[0];
        let new_out1_node = new_circuit.nets_at(new_out1_lit.net()).driver().unwrap();
        
        // Check Output 2
        let new_out2_lit = new_circuit.outputs()[1];
        let new_out2_node = new_circuit.nets_at(new_out2_lit.net()).driver().unwrap();

        // Both outputs should be driven by XOR3 gates now
        assert_eq!(*new_circuit.nodes_at(new_out1_node).gate(), Gate::Xor3);
        assert_eq!(*new_circuit.nodes_at(new_out2_node).gate(), Gate::Xor3);
        assert_eq!(new_circuit.nodes().len(), 2);
        assert_eq!(Simulator::compute_tt(&circuit), Simulator::compute_tt(&new_circuit));
    }

    #[test]
    fn test_xor_extraction_with_internal_fanout() {
        let mut circuit = Circuit::empty();
        
        let in_0 = circuit.add_input();
        let in_1 = circuit.add_input();

        let t0_inputs = vec![
            NetLit::new(in_0, false),
            NetLit::new(in_1, true),
        ];
        let t0 = circuit.add_gate(Gate::And, t0_inputs)[0];

        let t1_inputs = vec![
            NetLit::new(in_0, true),
            NetLit::new(in_1, false),
        ];
        let t1 = circuit.add_gate(Gate::And, t1_inputs)[0];

        let out_xor_inputs = vec![
            NetLit::new(t0, false),
            NetLit::new(t1, false),
        ];
        let out_xor = circuit.add_gate(Gate::Or, out_xor_inputs)[0];

        circuit.set_output(out_xor, false);
        circuit.set_output(t0, false); 

        let new_circuit = GenericExtractor::run(&circuit, XorExtractor);

        assert_eq!(new_circuit.inputs().len(), 2);
        assert_eq!(new_circuit.outputs().len(), 2);

        let out_xor_lit = new_circuit.outputs()[0];
        let out_side_lit = new_circuit.outputs()[1];

        let xor_driver = new_circuit.nets_at(out_xor_lit.net()).driver().unwrap();
        let xor_gate = new_circuit.nodes_at(xor_driver).gate();
        assert!(matches!(xor_gate, Gate::Xor));

        let side_driver = new_circuit.nets_at(out_side_lit.net()).driver().unwrap();
        let side_gate = new_circuit.nodes_at(side_driver).gate();
        assert!(matches!(side_gate, Gate::And));

        let side_node = new_circuit.nodes_at(side_driver);
        assert_eq!(side_node.inputs().len(), 2);
        assert_eq!(new_circuit.nodes().len(), 2);
        assert_eq!(Simulator::compute_tt(&circuit), Simulator::compute_tt(&new_circuit));
    }

    #[test]
    fn test_internal_consume() {
        // Actually, for a fully strashed circuit, this test is not needed.
        let mut circuit = Circuit::empty();
        
        let in_0 = circuit.add_input();
        let in_1 = circuit.add_input();

        // Construct: (in_0 & !in_1)
        let t0_inputs = vec![
            NetLit::new(in_0, false),
            NetLit::new(in_1, true),
        ];
        let t0 = circuit.add_gate(Gate::And, t0_inputs)[0];

        // Construct: (!in_0 & in_1)
        let t1_inputs = vec![
            NetLit::new(in_0, true),
            NetLit::new(in_1, false),
        ];
        let t1 = circuit.add_gate(Gate::And, t1_inputs)[0];

        // Construct: t0 | t1
        let internal_inputs = vec![
            NetLit::new(t0, false),
            NetLit::new(t1, false),
        ];
        let internal = circuit.add_gate(Gate::Or, internal_inputs)[0];

        // Consumed node: internal | t0
        let output_inputs = vec![
            NetLit::new(internal, false),
            NetLit::new(t0, false),
        ];
        let out = circuit.add_gate(Gate::Or, output_inputs)[0];

        circuit.set_output(out, false);

        // Run extractor
        let new_circuit = GenericExtractor::run(&circuit, XorExtractor);

        // Assertions
        assert_eq!(new_circuit.inputs().len(), 2);
        assert_eq!(new_circuit.outputs().len(), 1);

        // verify the single gate is XOR (assuming Gate::Xor) and the internal node is removed
        let out_net = new_circuit.outputs()[0].net();
        let driver_node = new_circuit.nets_at(out_net).driver().unwrap();
        assert_eq!(*new_circuit.nodes_at(driver_node).gate(), Gate::Xor);
        assert_eq!(new_circuit.nodes().len(), 1);
        assert_eq!(Simulator::compute_tt(&circuit), Simulator::compute_tt(&new_circuit));
    }

      #[test]
    fn test_cascade_xors() {
        // Actually, for a fully strashed circuit, this test is not needed.
        let mut circuit = Circuit::empty();
        
        let inputs: Vec<NetId>= (0..5).map(|_| circuit.add_input()).collect();

        let mut xor_input = inputs[0];
        for i in 1..5 {
            let xor_inputs = vec![
                NetLit::new(xor_input, false),
                NetLit::new(inputs[i], false),
            ];
            xor_input = circuit.add_gate(Gate::Xor, xor_inputs)[0];
        }

        circuit.set_output(xor_input, false);

        // Run extractor
        let new_circuit = GenericExtractor::run(&circuit, Xor3Extractor);

        // Assertions
        assert_eq!(new_circuit.inputs().len(), 5);
        assert_eq!(new_circuit.outputs().len(), 1);

        assert_eq!(new_circuit.nodes().len(), 2);
        assert_eq!(Simulator::compute_tt(&circuit), Simulator::compute_tt(&new_circuit));
    }

    #[test]
    fn test_simple_ha() {
        let mut circuit = Circuit::empty();
        let in_0 = circuit.add_input();
        let in_1 = circuit.add_input();

        let out_xor = circuit.add_gate(Gate::Xor, vec![
            NetLit::new(in_0, false),
            NetLit::new(in_1, false),
        ])[0];
        let out_and = circuit.add_gate(Gate::And, vec![
            NetLit::new(in_0, false),
            NetLit::new(in_1, false),
        ])[0];

        circuit.set_output(out_and, false);
        circuit.set_output(out_xor, false);

        let new_circuit = AdderExtractor::run(&circuit);
        assert_eq!(new_circuit.nodes().len(), 1);
        assert_eq!(*new_circuit.nodes_at(0).gate(), Gate::HalfAdder);
    }

    #[test]
    fn test_complex_ha() { 
        let mut circuit = Circuit::empty();
        let in_0 = circuit.add_input();
        let in_1 = circuit.add_input();

        let and_0 = circuit.add_gate(Gate::And, vec![
            NetLit::new(in_0, false),
            NetLit::new(in_1, true),
        ])[0];

        let and_1 = circuit.add_gate(Gate::And, vec![
            NetLit::new(in_0, true),
            NetLit::new(in_1, false),
        ])[0];

        let out_0 = circuit.add_gate(Gate::Or, vec![
            NetLit::new(and_0, false),
            NetLit::new(and_1, false),
        ])[0];
        
        circuit.set_output(and_0, false);
        circuit.set_output(out_0, false);

        let new_circuit = AdderExtractor::run(&circuit);

        // Assertions
        assert_eq!(new_circuit.nodes().len(), 1);
        assert_eq!(*new_circuit.nodes_at(0).gate(), Gate::HalfAdder);
    }

    #[test]
    fn test_more_complex_ha() { 
        let mut circuit = Circuit::empty();
        let in_0 = circuit.add_input();
        let in_1 = circuit.add_input();

        let and_0 = circuit.add_gate(Gate::And, vec![
            NetLit::new(in_0, false),
            NetLit::new(in_1, true),
        ])[0];

        let and_1 = circuit.add_gate(Gate::And, vec![
            NetLit::new(in_0, true),
            NetLit::new(in_1, false),
        ])[0];

        let and_2 = circuit.add_gate(Gate::And, vec![
            NetLit::new(in_0, false),
            NetLit::new(in_1, false),
        ])[0];

        let out_0 = circuit.add_gate(Gate::Or, vec![
            NetLit::new(and_0, false),
            NetLit::new(and_1, false),
        ])[0];
        
        circuit.set_output(and_2, false);
        circuit.set_output(out_0, false);

        let new_circuit = AdderExtractor::run(&circuit);

        // Assertions
        assert_eq!(new_circuit.nodes().len(), 1);
        assert_eq!(*new_circuit.nodes_at(0).gate(), Gate::HalfAdder);
    }

    #[test]
    fn test_simple_fa() { 
        let mut circuit = Circuit::empty();
        let in_0 = circuit.add_input();
        let in_1 = circuit.add_input();
        let in_2 = circuit.add_input();

        let out_xor3 = circuit.add_gate(Gate::Xor3, vec![
            NetLit::new(in_0, false),
            NetLit::new(in_1, false),
            NetLit::new(in_2, false),
        ])[0];

        let out_maj = circuit.add_gate(Gate::Maj, vec![
            NetLit::new(in_0, false),
            NetLit::new(in_1, false),
            NetLit::new(in_2, false),
        ])[0];

        circuit.set_output(out_xor3, false);
        circuit.set_output(out_maj, false);

        let new_circuit = AdderExtractor::run(&circuit);
        assert_eq!(new_circuit.nodes().len(), 1);
        assert_eq!(*new_circuit.nodes_at(0).gate(), Gate::FullAdder);
    }

    #[test]
    fn test_fa_containing_ha() {
        let mut circuit = Circuit::empty();
        
        // 1. Inputs: A, B, Cin
        let a = circuit.add_input();
        let b = circuit.add_input();
        let cin = circuit.add_input();

        // 2. First Half Adder Structure (Inner HA)
        // HA1_Sum = A ^ B
        let ha1_sum = circuit.add_gate(Gate::Xor, vec![
            NetLit::new(a, false),
            NetLit::new(b, false),
        ])[0];
        
        // HA1_Carry = A & B
        let ha1_carry = circuit.add_gate(Gate::And, vec![
            NetLit::new(a, false),
            NetLit::new(b, false),
        ])[0];

        // 3. Second Half Adder Structure Logic
        // FA_Sum = HA1_Sum ^ Cin
        let fa_sum = circuit.add_gate(Gate::Xor, vec![
            NetLit::new(ha1_sum, false),
            NetLit::new(cin, false),
        ])[0];

        // HA2_Carry = HA1_Sum & Cin
        let ha2_carry = circuit.add_gate(Gate::And, vec![
            NetLit::new(ha1_sum, false),
            NetLit::new(cin, false),
        ])[0];

        // 4. Final Carry Logic
        // FA_Carry = HA1_Carry | HA2_Carry
        let fa_carry = circuit.add_gate(Gate::Or, vec![
            NetLit::new(ha1_carry, false),
            NetLit::new(ha2_carry, false),
        ])[0];

        circuit.set_output(fa_sum, false);
        circuit.set_output(fa_carry, false);

        // Run the extractor
        let new_circuit = AdderExtractor::run(&circuit);

        // Verification
        assert_eq!(new_circuit.nodes().len(), 1);
        assert_eq!(*new_circuit.nodes_at(0).gate(), Gate::FullAdder);

        assert_eq!(Simulator::compute_tt(&circuit), Simulator::compute_tt(&new_circuit));
    }

    #[test]
    fn test_fa_containing_ha_extrafanout() {
        let mut circuit = Circuit::empty();
        
        // 1. Inputs: A, B, Cin
        let a = circuit.add_input();
        let b = circuit.add_input();
        let cin = circuit.add_input();

        // 2. First Half Adder Structure (Inner HA)
        // HA1_Sum = A ^ B
        let ha1_sum = circuit.add_gate(Gate::Xor, vec![
            NetLit::new(a, false),
            NetLit::new(b, false),
        ])[0];
        
        // HA1_Carry = A & B
        let ha1_carry = circuit.add_gate(Gate::And, vec![
            NetLit::new(a, false),
            NetLit::new(b, false),
        ])[0];

        // 3. Second Half Adder Structure Logic
        // FA_Sum = HA1_Sum ^ Cin
        let fa_sum = circuit.add_gate(Gate::Xor, vec![
            NetLit::new(ha1_sum, false),
            NetLit::new(cin, false),
        ])[0];

        // HA2_Carry = HA1_Sum & Cin
        let ha2_carry = circuit.add_gate(Gate::And, vec![
            NetLit::new(ha1_sum, false),
            NetLit::new(cin, false),
        ])[0];

        // 4. Final Carry Logic
        // FA_Carry = HA1_Carry | HA2_Carry
        let fa_carry = circuit.add_gate(Gate::Or, vec![
            NetLit::new(ha1_carry, false),
            NetLit::new(ha2_carry, false),
        ])[0];

        circuit.set_output(fa_sum, false);
        circuit.set_output(fa_carry, false);
        circuit.set_output(ha1_sum, false);
        circuit.set_output(ha1_carry, false);

        // Run the extractor
        let new_circuit = AdderExtractor::run(&circuit);

        // Verification
        assert_eq!(new_circuit.nodes().len(), 3);

        let ha1 = new_circuit.nodes_at(new_circuit.nets_at(new_circuit.outputs()[0].net()).driver().unwrap());
        assert_eq!(*ha1.gate(), Gate::HalfAdder);

        let ha2 = new_circuit.nodes_at(new_circuit.nets_at(new_circuit.outputs()[2].net()).driver().unwrap());
        assert_eq!(*ha2.gate(), Gate::HalfAdder);

        let or = new_circuit.nodes_at(new_circuit.nets_at(new_circuit.outputs()[1].net()).driver().unwrap());
        assert_eq!(*or.gate(), Gate::Or);

        assert_eq!(Simulator::compute_tt(&circuit), Simulator::compute_tt(&new_circuit));
    }

    #[test]
    fn test_fa_containing_ha_incomplete_extrafanout() {
        let mut circuit = Circuit::empty();
        
        // 1. Inputs: A, B, Cin
        let a = circuit.add_input();
        let b = circuit.add_input();
        let cin = circuit.add_input();

        // 2. First Half Adder Structure (Inner HA)
        // HA1_Sum = A ^ B
        let ha1_sum = circuit.add_gate(Gate::Xor, vec![
            NetLit::new(a, false),
            NetLit::new(b, false),
        ])[0];
        
        // HA1_Carry = A & B
        let ha1_carry = circuit.add_gate(Gate::And, vec![
            NetLit::new(a, false),
            NetLit::new(b, false),
        ])[0];

        // 3. Second Half Adder Structure Logic
        // FA_Sum = HA1_Sum ^ Cin
        let fa_sum = circuit.add_gate(Gate::Xor, vec![
            NetLit::new(ha1_sum, false),
            NetLit::new(cin, false),
        ])[0];

        // HA2_Carry = HA1_Sum & Cin
        let ha2_carry = circuit.add_gate(Gate::And, vec![
            NetLit::new(ha1_sum, false),
            NetLit::new(cin, false),
        ])[0];

        // 4. Final Carry Logic
        // FA_Carry = HA1_Carry | HA2_Carry
        let fa_carry = circuit.add_gate(Gate::Or, vec![
            NetLit::new(ha1_carry, false),
            NetLit::new(ha2_carry, false),
        ])[0];

        circuit.set_output(fa_sum, false);
        circuit.set_output(fa_carry, false);
        circuit.set_output(ha1_sum, false);

        // Run the extractor;
        let new_circuit = AdderExtractor::run(&circuit);

        // // Verification
        assert_eq!(new_circuit.nodes().len(), 3);

        let ha1 = new_circuit.nodes_at(new_circuit.nets_at(new_circuit.outputs()[0].net()).driver().unwrap());
        assert_eq!(*ha1.gate(), Gate::HalfAdder);

        let ha2 = new_circuit.nodes_at(new_circuit.nets_at(new_circuit.outputs()[2].net()).driver().unwrap());
        assert_eq!(*ha2.gate(), Gate::HalfAdder);

        let or = new_circuit.nodes_at(new_circuit.nets_at(new_circuit.outputs()[1].net()).driver().unwrap());
        assert_eq!(*or.gate(), Gate::Or);

        assert_eq!(Simulator::compute_tt(&circuit), Simulator::compute_tt(&new_circuit));
    }

    #[test]
    fn test_fa_output_negated() {
        let mut circuit = Circuit::empty();

        // 1. Inputs: A, B, Cin
        let a = circuit.add_input();
        let b = circuit.add_input();
        let cin = circuit.add_input();
        let a2 = circuit.add_input();
        let b2 = circuit.add_input();

        // 2. internal gates
        let maj = circuit.add_gate(Gate::Maj, vec![
            NetLit::new(a, false),
            NetLit::new(b, false),
            NetLit::new(cin, true),
        ])[0];
        
        let xor3 = circuit.add_gate(Gate::Xor3, vec![
            NetLit::new(a, true),
            NetLit::new(b, true),
            NetLit::new(cin, false),
        ])[0];

        let maj2 = circuit.add_gate(Gate::Maj, vec![
            NetLit::new(a2, false),
            NetLit::new(b2, true),
            NetLit::new(maj, false),
        ])[0];

        let xor32 = circuit.add_gate(Gate::Xor3, vec![
            NetLit::new(a2, true),
            NetLit::new(b2, false),
            NetLit::new(maj, true),
        ])[0];

        circuit.set_output(xor3, true);
        circuit.set_output(xor32, false);
        circuit.set_output(maj2, true);

        let new_circuit = AdderExtractor::run(&circuit);

        assert_eq!(new_circuit.nodes().len(), 2);
        assert_eq!(Simulator::compute_tt(&circuit), Simulator::compute_tt(&new_circuit));
    }
}
