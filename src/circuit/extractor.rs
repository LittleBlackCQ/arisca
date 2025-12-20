use super::*;
use std::collections::{BTreeSet, HashMap};

pub trait ExtractorStrategy {
    fn cut_size(&self) -> usize;
    fn target_gate(&self) -> Gate;
    fn match_tt(&self, tt: &[bool]) -> Option<(bool, Vec<bool>)>;
}

#[derive(Debug, Clone)]
pub struct Match {
    pub root: NetId,
    pub cut: Vec<NetId>,
    pub cone_nets: BTreeSet<NetId>,
    pub cone_nodes: BTreeSet<NodeId>,
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
        covered_nodes: &BTreeSet<NetId>
    ) -> Option<Match> {
        if cut.len() != strategy.cut_size() { return None; }

        let mut cone_nets = BTreeSet::new();
        let mut cone_nodes = BTreeSet::new();
        let mut is_valid = true;
        
        circuit.bfs_cone(root, true, |n| {
            cone_nets.insert(n);
            if cut.contains(&n) { return false; }
            if let Some(id) = circuit.nets()[n].driver() {
                if covered_nodes.contains(&n) { is_valid = false; }
                cone_nodes.insert(id);
            }
            is_valid
        });
        
        if !is_valid { return None; }

        let inputs: Vec<NetId> = cut.iter().copied().collect();
        let subcircuit = circuit.subcircuit(
            &cone_nets.iter().copied().collect::<Vec<_>>(),
            &cone_nodes.iter().copied().collect::<Vec<_>>(),
            &inputs,
            &[NetLit::new(root, false)],
        );

        let tt: Vec<bool> = subcircuit.get_tt().iter().map(|r| r[0]).collect();
        strategy.match_tt(&tt).map(|(neg, in_negs)| Match { 
            root, cut: inputs, cone_nets, cone_nodes, 
            output_negated: neg, input_negations: in_negs, gate: strategy.target_gate(),
        })
    }

    pub fn mark_retained_logic(
        circuit: &Circuit, 
        matches: &[&Match], 
        kept_nets: &mut BTreeSet<NetId>, 
        kept_nodes: &mut BTreeSet<usize>
    ) {
        let all_roots: BTreeSet<NetId> = matches.iter().map(|m| m.root).collect();
        let all_cone_nodes: BTreeSet<usize> = matches.iter().flat_map(|m| &m.cone_nodes).copied().collect();
        
        let retained_roots: Vec<NetId> = matches.iter().flat_map(|m| &m.cone_nets)
            .filter(|&&n| {
                if all_roots.contains(&n) { return false; }
                circuit.nets()[n].loads().iter().any(|&o| !all_cone_nodes.contains(&o))
            })
            .copied()
            .collect();

        for n in retained_roots {
            circuit.bfs_cone(n, true, |net| {
                if !kept_nets.insert(net) { return false; }
                if let Some(nid) = circuit.nets()[net].driver() { kept_nodes.insert(nid); }
                true
            });
        }
    }

    pub fn run<S: ExtractorStrategy>(circuit: &Circuit, strategy: S) -> Circuit {
        let cut_db = circuit.get_cuts(strategy.cut_size(), 20);
        let mut matches = Vec::new();
        let mut covered = BTreeSet::new();

        for &root in circuit.topology_order().iter() {
            if covered.contains(&root) { continue; }
            for cut in &cut_db[root] {
                if let Some(m) = Self::try_match(circuit, root, cut, &strategy, &covered) {
                    for &n in &m.cone_nets {
                        if n != m.root && !m.cut.contains(&n) { covered.insert(n); }
                    }
                    matches.push(m);
                    break;
                }
            }
        }
        Self::rebuild_circuit(circuit, matches)
    }

    pub fn rebuild_circuit(circuit: &Circuit, matches: Vec<Match>) -> Circuit {
        let mut kept_nets = BTreeSet::new();
        let mut kept_nodes = BTreeSet::new();

        for m in &matches {
            kept_nets.extend(m.cut.iter());
            kept_nets.insert(m.root);
            Self::mark_retained_logic(circuit, &[m], &mut kept_nets, &mut kept_nodes);
        }

        let negated_outputs: Vec<NetId> = matches.iter()
            .filter_map(|m| if m.output_negated { Some(m.root) } else { None }).collect();
    
        let mut net_map = HashMap::new();
        net_map.insert(0, 0); 
        let mut new_nets = Vec::new();
        let mut new_nodes = Vec::new();

        for n in 0..circuit.nets().len() {
            let in_cone = matches.iter().any(|m| m.cone_nets.contains(&n));
            if !in_cone || kept_nets.contains(&n) {
                net_map.insert(n, new_nets.len());
                new_nets.push(Net::empty());
            }
        }

        for (nid, node) in circuit.nodes().iter().enumerate() {
            let in_cone = matches.iter().any(|m| m.cone_nodes.contains(&nid));
            if !in_cone || kept_nodes.contains(&nid) {
                let ins = node.inputs().iter().map(|l| {
                    let map_n = net_map.get(&l.net()).expect("Net missing");
                    NetLit::new(*map_n, l.negative() ^ negated_outputs.contains(&l.net()))
                }).collect();
                let outs = node.outputs().iter().map(|o| net_map[o]).collect();
                
                let id = new_nodes.len();
                new_nodes.push(Node::new(node.name().clone(), node.gate().clone(), ins, outs));
                for o in node.outputs() { new_nets[net_map[o]].set_driver(id); }
                for l in node.inputs() { new_nets[net_map[&l.net()]].add_load(id); }
            }
        }

        for m in &matches {
            let ins: Vec<NetLit> = m.cut.iter().zip(m.input_negations.iter()).map(|(&n, &neg)| 
                NetLit::new(net_map[&n], negated_outputs.contains(&n) ^ neg)
            ).collect();

            let o = net_map[&m.root];
            let id = new_nodes.len();
            new_nodes.push(Node::new(None, m.gate, ins, vec![o]));
            for inp in &m.cut { new_nets[net_map[inp]].add_load(id); }
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
        match tt {
            [false, true, true, false] => Some((false, vec![false; 2])),
            [true, false, false, true] => Some((true, vec![false; 2])),
            _ => None
        }
    }
}
impl XorExtractor { pub fn run(c: &Circuit) -> Circuit { GenericExtractor::run(c, XorExtractor) } }

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
impl Xor3Extractor { pub fn run(c: &Circuit) -> Circuit { GenericExtractor::run(c, Xor3Extractor) } }

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
impl MajExtractor { pub fn run(c: &Circuit) -> Circuit { GenericExtractor::run(c, MajExtractor) } }

// --- Adder Extractor (Multi-output) ---

pub struct AdderExtractor;
impl AdderExtractor {
    pub fn run(circuit: &Circuit) -> Circuit {
        let cut_db = circuit.get_cuts(3, 20);
        let mut candidates: HashMap<Vec<NetId>, Vec<Match>> = HashMap::new();
        let strategies: [&dyn ExtractorStrategy; 4] = [
            &Xor3Extractor, &MajExtractor,
            &XorExtractor, &AndExtractor,
        ];

        for &root in circuit.topology_order().iter() {
            for cut in &cut_db[root] {
                for strategy in strategies {
                    if let Some(m) = GenericExtractor::try_match(circuit, root, cut, strategy, &BTreeSet::new()) {
                        candidates.entry(m.cut.clone()).or_default().push(m);
                    }
                }
            }
        }

        let mut matches = Vec::new();
        let mut covered = BTreeSet::new();
        let mut sorted_cuts: Vec<_> = candidates.keys().cloned().collect();
        sorted_cuts.sort_by(|a, b| b.len().cmp(&a.len()));

        for cut in sorted_cuts {
            let ms = &candidates[&cut];
            let xor3s: Vec<&Match> = ms.iter().filter(|m| m.gate == Gate::Xor3).collect();
            let majs: Vec<&Match> = ms.iter().filter(|m| m.gate == Gate::Maj).collect();
            let xors: Vec<&Match> = ms.iter().filter(|m| m.gate == Gate::Xor).collect();
            let ands: Vec<&Match> = ms.iter().filter(|m| m.gate == Gate::And).collect();
            
            let mut find_adder = |sums: &[&Match], carries: &[&Match], gate_type: Gate| -> bool {
                for s in sums {
                    for c in carries {
                        if Self::is_valid(s, &covered) 
                            && Self::is_valid(c, &covered) 
                            && Self::check_fanout(circuit, s, c) 
                        {
                            Self::mark_covered(s, &mut covered);
                            Self::mark_covered(c, &mut covered);
                            matches.push(AdderMatch { 
                                sum: (*s).clone(), 
                                carry: (*c).clone(), 
                                gate: gate_type 
                            });
                            return true;
                        }
                    }
                }
                false
            };

            let found = if cut.len() == 3 {
                find_adder(&xor3s, &majs, Gate::FullAdder)
            } else if cut.len() == 2 {
                find_adder(&xors, &ands, Gate::HalfAdder)
            } else { false };

            if found { continue; }
        }
        Self::rebuild_adders(circuit, matches)
    }

    fn check_fanout(circuit: &Circuit, s: &Match, c: &Match) -> bool {
        if s.cone_nets.contains(&c.root) {
             let is_internal_only = circuit.nets()[c.root].loads().iter()
            .all(|n| s.cone_nodes.contains(n));
             if is_internal_only { return false; }
        }
        true
    }

    fn is_valid(m: &Match, covered: &BTreeSet<NetId>) -> bool {
        if covered.contains(&m.root) { return false; }
        m.cone_nets.iter().all(|&n| n == m.root || m.cut.contains(&n) || !covered.contains(&n))
    }

    fn mark_covered(m: &Match, covered: &mut BTreeSet<NetId>) {
        covered.insert(m.root);
        for &n in &m.cone_nets {
            if n != m.root && !m.cut.contains(&n) { covered.insert(n); }
        }
    }

    fn rebuild_adders(circuit: &Circuit, matches: Vec<AdderMatch>) -> Circuit {
        let mut kept_nets = BTreeSet::new();
        let mut kept_nodes = BTreeSet::new();

        for am in &matches {
            let m_list = [&am.sum, &am.carry];
            for m in m_list {
                kept_nets.extend(m.cut.iter());
                kept_nets.insert(m.root);
            }
            GenericExtractor::mark_retained_logic(circuit, &m_list, &mut kept_nets, &mut kept_nodes);
        }
        
        let mut out_mods = HashMap::new();
        for am in &matches {
            let s = &am.sum;
            let c = &am.carry;
            
            if c.output_negated { out_mods.insert(c.root, true); }

            let parity_diff = s.input_negations.iter().zip(c.input_negations.iter())
                .filter(|(a, b)| a != b).count() % 2 != 0;
            
            if s.output_negated ^ parity_diff {
                out_mods.insert(s.root, true);
            }
        }

        let mut net_map = HashMap::new();
        net_map.insert(0, 0);
        let mut new_nets = Vec::new();
        let mut new_nodes = Vec::new();

        for n in 0..circuit.nets().len() {
            let in_cone = matches.iter().any(|am| {
                am.sum.cone_nets.contains(&n) || am.carry.cone_nets.contains(&n)
            });
            if !in_cone || kept_nets.contains(&n) {
                net_map.insert(n, new_nets.len());
                new_nets.push(Net::empty());
            }
        }
        
        for (nid, node) in circuit.nodes().iter().enumerate() {
            let in_cone = matches.iter().any(|am| {
                am.sum.cone_nodes.contains(&nid) || am.carry.cone_nodes.contains(&nid)
            });
            
            if !in_cone || kept_nodes.contains(&nid) {
                 let ins = node.inputs().iter().map(|l| {
                    let map_n = net_map.get(&l.net()).expect("Net missing");
                    NetLit::new(*map_n, l.negative() ^ out_mods.contains_key(&l.net()))
                }).collect();
                let outs = node.outputs().iter().map(|o| net_map[o]).collect();
                
                let id = new_nodes.len();
                new_nodes.push(Node::new(node.name().clone(), node.gate().clone(), ins, outs));
                for o in node.outputs() { new_nets[net_map[o]].set_driver(id); }
                for l in node.inputs() { new_nets[net_map[&l.net()]].add_load(id); }
            }
        }

        for am in &matches {
            let ref_match = &am.carry;
            let outs = vec![am.sum.root, am.carry.root];
            
            let in_lits: Vec<NetLit> = ref_match.cut.iter().zip(ref_match.input_negations.iter()).map(|(&n, &neg)| 
                NetLit::new(net_map[&n], out_mods.contains_key(&n) ^ neg)
            ).collect();
            let out_ids: Vec<usize> = outs.iter().map(|n| net_map[n]).collect();

            let id = new_nodes.len();
            new_nodes.push(Node::new(None, am.gate, in_lits, out_ids.clone()));
            for inp in &ref_match.cut { new_nets[net_map[inp]].add_load(id); }
            for &o in &out_ids { new_nets[o].set_driver(id); }
        }

        Circuit::new(new_nodes, new_nets,
            circuit.inputs().iter().map(|m| net_map[m]).collect(), 
            circuit.outputs().iter().map(|m| NetLit::new(net_map[&m.net()], m.negative() ^ out_mods.contains_key(&m.net()))).collect())
    }
}

struct AdderMatch {
    sum: Match,
    carry: Match,
    gate: Gate,
}