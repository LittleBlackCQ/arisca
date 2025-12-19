use super::*;
use std::collections::{BTreeSet, HashMap};

pub trait ExtractorStrategy {
    fn cut_size(&self) -> usize;
    fn target_gate(&self) -> Gate;
    // returns Some((output_negated, input_negations)) if matched
    fn match_tt(&self, tt: &[bool]) -> Option<(bool, Vec<bool>)>;
}

pub struct GenericExtractor;

impl GenericExtractor {
    pub fn run<S: ExtractorStrategy>(circuit: &Circuit, strategy: S) -> Circuit {
        struct Match {
            root: NetId,
            cut: Vec<NetId>,
            cone_nets: BTreeSet<NetId>,
            cone_nodes: BTreeSet<usize>,
            output_negated: bool,
            input_negations: Vec<bool>,
        }

        let cut_db = circuit.get_cuts(strategy.cut_size(), 8);
        let mut matches = Vec::new();
        let mut covered_nodes = BTreeSet::new();

        // Since the matching is greedy, so the order matters.
        // Here we simply use the topology order of the nets.
        for &root in circuit.topology_order().iter() {
            if covered_nodes.contains(&root) { continue; } // Naive overlap check

            for cut in &cut_db[root] {
                if cut.len() != strategy.cut_size() { continue; }

                // BFS cone
                let mut cone_nets = BTreeSet::new();
                let mut cone_nodes = BTreeSet::new();
                let mut is_valid_cone = true;
                
                circuit.bfs_cone(root, true, |n| {
                    cone_nets.insert(n);
                    if cut.contains(&n) { return false; }
                    if let Some(id) = circuit.nets()[n].driver() {
                        if covered_nodes.contains(&n) {
                            is_valid_cone = false;
                        }
                        cone_nodes.insert(id);
                    }
                    is_valid_cone
                });
                
                if !is_valid_cone { continue; }

                let inputs: Vec<NetId> = cut.iter().copied().collect();
                let sub = circuit.subcircuit(
                    &cone_nets.iter().copied().collect::<Vec<_>>(),
                    &cone_nodes.iter().copied().collect::<Vec<_>>(),
                    &inputs,
                    &[NetLit::new(root, false)],
                );

                let tt: Vec<bool> = sub.get_tt().iter().map(|r| r[0]).collect();
                if let Some((output_negated, input_negations)) = strategy.match_tt(&tt) {
                    matches.push(Match { 
                        root, 
                        cut: inputs, 
                        cone_nets: cone_nets.clone(), 
                        cone_nodes: cone_nodes.clone(), 
                        output_negated,
                        input_negations,
                    });
                    
                    // Mark nets as covered to avoid overlapping extractions
                    // Only mark strictly internal nodes. 
                    for &n in &cone_nets {
                        if n != root && !cut.contains(&n) {
                            covered_nodes.insert(n);
                        }
                    }
                    break;
                }
            }
        }

        let mut kept_nets = BTreeSet::new();
        let mut kept_nodes = BTreeSet::new();

        // Calculate retained logic
        for m in &matches {
            kept_nets.extend(m.cut.iter());
            kept_nets.insert(m.root);
            
            // Re-traverse from inputs/roots that are NOT part of the replaced cones
            let roots = m.cone_nets.iter()
                .filter(|&&net_id| circuit.nets()[net_id].loads().iter()
                .any(|&o| !m.cone_nodes.contains(&o))
                ).copied().collect::<Vec<_>>();

            for net_id in roots {
                // Backward BFS to keep drivers of retained nets
                circuit.bfs_cone(net_id, true, |net| {
                    if !kept_nets.insert(net) { return false; }
                    if let Some(node_id) = circuit.nets()[net].driver() {
                        kept_nodes.insert(node_id);
                    }
                    true
                });
            }
        }

        let negated_outputs: Vec<NetId> = matches.iter()
            .filter_map(|m| if m.output_negated { Some(m.root) } else { None })
            .collect();
    
        let mut net_map = HashMap::new();
        net_map.insert(0, 0); // constant 0
        let mut new_nets = Vec::new();
        let mut new_nodes = Vec::new();

        // Build new nets
        for n in 0..circuit.nets().len() {
            let in_match_cone = matches.iter().any(|m| m.cone_nets.contains(&n));
            let keep = !in_match_cone || kept_nets.contains(&n);
            
            if keep {
                net_map.insert(n, new_nets.len());
                new_nets.push(Net::empty());
            }
        }

        // Rebuild existing nodes
        for (nid, node) in circuit.nodes().iter().enumerate() {
            let in_match_cone = matches.iter().any(|m| m.cone_nodes.contains(&nid));
            let keep = !in_match_cone || kept_nodes.contains(&nid);

            if keep {
                let ins = node.inputs().iter().map(|l| {
                    let mapped_net = net_map.get(&l.net()).expect(&format!(
                        "Net {} not found in map! It is an input to kept node {}, but the net itself was not kept. \
                        This usually means a logic cone was removed but an internal wire had external fanout.", 
                        l.net(), nid
                    ));
                    NetLit::new(*mapped_net, l.negative() ^ negated_outputs.contains(&l.net()))
                }).collect();
                let outs = node.outputs().iter().map(|o| net_map[o]).collect();
                
                let id = new_nodes.len();
                new_nodes.push(Node::new(node.name().clone(), node.gate().clone(), ins, outs));
                for o in node.outputs() { new_nets[net_map[o]].set_driver(id); }
                for l in node.inputs() { new_nets[net_map[&l.net()]].add_load(id); }
            }
        }

        // Add extracted nodes
        for m in &matches {
            // Apply input negations from NPN match
            let ins: Vec<NetLit> = m.cut.iter().zip(m.input_negations.iter()).map(|(&n, &neg)| 
                NetLit::new(net_map[&n], negated_outputs.contains(&n) ^ neg)
            ).collect();

            let o = net_map[&m.root];
            let id = new_nodes.len();
            
            new_nodes.push(Node::new(None, strategy.target_gate(), ins, vec![o]));
            
            for inp in &m.cut {
                new_nets[net_map[inp]].add_load(id);
            }
            new_nets[o].set_driver(id);
        }

        Circuit::new(new_nodes, new_nets,
            circuit.inputs().iter().map(|m| net_map[m]).collect(), 
            circuit.outputs().iter().map(|m| NetLit::new(net_map[&m.net()], m.negative() ^ negated_outputs.contains(&m.net()))).collect())
    }
}

// --- Specific Strategies ---

pub struct XorExtractor;
impl ExtractorStrategy for XorExtractor {
    fn cut_size(&self) -> usize { 2 }
    fn target_gate(&self) -> Gate { Gate::Xor }
    fn match_tt(&self, tt: &[bool]) -> Option<(bool, Vec<bool>)> {
        let xor_tt = [false, true, true, false]; // 0110
        let xnor_tt = [true, false, false, true]; // 1001
        
        if tt == &xor_tt { Some((false, vec![false, false])) }
        else if tt == &xnor_tt { Some((true, vec![false, false])) }
        else { None }
    }
}
impl XorExtractor {
    pub fn run(circuit: &Circuit) -> Circuit {
        GenericExtractor::run(circuit, XorExtractor)
    }
}

pub struct Xor3Extractor;
impl ExtractorStrategy for Xor3Extractor {
    fn cut_size(&self) -> usize { 3 }
    fn target_gate(&self) -> Gate { Gate::Xor3 }
    fn match_tt(&self, tt: &[bool]) -> Option<(bool, Vec<bool>)> {
        // a^b^c map: 0->0, 1->1, 2->1, 3->0, 4->1, 5->0, 6->0, 7->1
        let xor3_tt = [false, true, true, false, true, false, false, true]; 
        let xnor3_tt = [true, false, false, true, false, true, true, false]; 
        
        if tt == &xor3_tt { Some((false, vec![false, false, false])) }
        else if tt == &xnor3_tt { Some((true, vec![false, false, false])) }
        else { None }
    }
}
impl Xor3Extractor {
    pub fn run(circuit: &Circuit) -> Circuit {
        GenericExtractor::run(circuit, Xor3Extractor)
    }
}

pub struct MajExtractor;
impl ExtractorStrategy for MajExtractor {
    fn cut_size(&self) -> usize { 3 }
    fn target_gate(&self) -> Gate { Gate::Maj }
    fn match_tt(&self, tt: &[bool]) -> Option<(bool, Vec<bool>)> {
        // Check all NPN input-negation classes for MAJ
        // MAJ is self-dual: !MAJ(a,b,c) == MAJ(!a,!b,!c). 
        // We iterate through all 8 input masks to find a match.
        for mask in 0..8 {
            let negs: Vec<bool> = (0..3).map(|i| (mask >> i) & 1 == 1).collect();
            
            // Generate expected TT for MAJ with these input negations
            let expected_tt: Vec<bool> = (0..8).map(|i| {
                let mut count = 0;
                for bit in 0..3 {
                    // Get input bit and apply mask negation
                    let val = ((i >> bit) & 1 == 1) ^ negs[bit];
                    if val { count += 1; }
                }
                count >= 2
            }).collect();

            if tt == &expected_tt {
                return Some((false, negs));
            }
            
            // Also check inverted output. 
            // Note: Since MAJ covers its own inverse via input negations, this branch
            // technically just finds the same function with complementary input mask,
            // but it's kept for robustness.
            let inverted_tt: Vec<bool> = expected_tt.iter().map(|&b| !b).collect();
            if tt == &inverted_tt {
                 return Some((true, negs));
            }
        }
        None
    }
}
impl MajExtractor {
    pub fn run(circuit: &Circuit) -> Circuit {
        GenericExtractor::run(circuit, MajExtractor)
    }
}