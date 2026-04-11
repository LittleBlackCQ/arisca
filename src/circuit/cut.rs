use super::*;

use std::collections::BTreeSet;

pub type Cut = BTreeSet<NetId>;
pub type CutDb = Vec<Vec<Cut>>;

pub struct CutEnumerator {
    cut_size: usize,
    cut_limit: usize,
}

impl CutEnumerator {
    pub fn new(cut_size: usize, cut_limit: usize) -> Self {
        Self {
            cut_size,
            cut_limit,
        }
    }

    fn merge_two(a: &Cut, b: &Cut) -> Cut {
        a.union(b).cloned().collect()
    }

    fn is_dominated(candidate: &Cut, existing: &[Cut]) -> bool {
        existing.iter().any(|c| c.is_subset(candidate))
    }

    fn merge_cuts(&self, c1: &[Cut], c2: &[Cut]) -> Vec<Cut> {
        let mut result = Vec::<Cut>::new();

        for a in c1 {
            for b in c2 {
                let merged = Self::merge_two(a, b);
                if merged.len() > self.cut_size {
                    continue;
                }
                if !Self::is_dominated(&merged, &result) {
                    result.push(merged);
                }
            }
        }

        result.sort_by_key(|c| c.len());
        result.truncate(self.cut_limit);
        result
    }

    fn singleton_cut(net: NetId) -> Cut {
        let mut c = BTreeSet::new();
        c.insert(net);
        c
    }

    pub fn run(&self, circuit: &Circuit) -> CutDb {
        let net_num = circuit.nets().len();
        let mut cuts: Vec<Vec<Cut>> = vec![Vec::new(); net_num];

        for net in 0..circuit.nets().len() {
            if circuit.inputs().contains(&net) {
                // PI
                cuts[net].push(Self::singleton_cut(net));
                continue;
            }

            if let Some(node_id) = circuit.nets_at(net).driver() {
                let node = circuit.nodes_at(node_id);
                if node.outputs().len() > 1 {
                    cuts[net].push(Self::singleton_cut(net));
                    continue;
                }
                let inputs = node.inputs();

                if inputs.is_empty() {
                    cuts[net].push(Cut::new());
                } else {
                    let mut new_cuts = cuts[inputs[0].net()].clone();

                    for lit in &inputs[1..] {
                        new_cuts = self.merge_cuts(&new_cuts, &cuts[lit.net()]);
                        if new_cuts.is_empty() {
                            break;
                        }
                    }
                    cuts[net] = new_cuts;
                }

                // add unit cut
                cuts[net].push(Self::singleton_cut(net));
            } else {
                cuts[net].push(Cut::new());
            }
        }
        cuts
    }
}

impl Circuit {
    pub fn get_cuts(&self, cut_size: usize, cut_limit: usize) -> CutDb {
        CutEnumerator::new(cut_size, cut_limit).run(self)
    }
}
