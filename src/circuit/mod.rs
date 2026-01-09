pub mod basics;
pub mod gate;
pub mod debug;
pub mod extractor;
pub mod cut;
pub mod sim;

pub use crate::circuit::basics::{Node, Net, NetId, NodeId, NetLit};
pub use crate::circuit::gate::Gate;
use std::collections::{VecDeque};

pub struct Circuit {
    nodes: Vec<Node>,
    nets: Vec<Net>,
    inputs: Vec<NetId>,
    outputs: Vec<NetLit>,
}

impl Circuit {

    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    pub fn nodes_at(&self, i: usize) -> &Node {
        &self.nodes[i]
    }

    pub fn nets(&self) -> &[Net] {
        &self.nets
    }

    pub fn nets_at(&self, i: usize) -> &Net {
        &self.nets[i]
    }

    pub fn inputs(&self) -> &[NetId] {
        &self.inputs
    }

    pub fn outputs(&self) -> &[NetLit] {
        &self.outputs
    }

    pub fn is_multioutput(&self) -> bool {
        for node in self.nodes() {
            if node.outputs().len() > 1 {
                return true;
            }
        }
        false
    }

    pub fn topology_nets(&self) -> Vec<NetId> {
        let net_num = self.nets.len();
        let mut indegree: Vec<NetId> = vec![0; net_num];

        let mut adj: Vec<Vec<NetId>> = vec![Vec::<NetId>::new(); net_num];

        for node in self.nodes() {
            for out in node.outputs() {
                for inp in node.inputs() {
                    let inp_net = inp.net();
                    adj[inp_net].push(*out);
                    indegree[*out] += 1;
                }
            }
        }

        for node in self.nodes() {
            let outputs = node.outputs();
            let topo = node.gate().output_topology();

            // topo: [i0, i1, i2, ...]
            // meaning: outputs[i0] -> outputs[i1] -> ...
            for w in topo.windows(2) {
                let from = outputs[w[0]];
                let to   = outputs[w[1]];
                adj[from].push(to);
                indegree[to] += 1;
            }
        }

        let mut queue = VecDeque::new();
        for net in 0..net_num {
            if indegree[net] == 0 {
                queue.push_back(net);
            }
        }

        let mut order = Vec::with_capacity(net_num);
        while let Some(n) = queue.pop_front() {
            order.push(n);
            let driver_n = self.nets_at(n).driver();

            for &succ in &adj[n] {
                indegree[succ] -= 1;
                if indegree[succ] == 0 {
                    let is_sibling = driver_n.is_some()
                        && driver_n == self.nets_at(succ).driver();
                    
                    if is_sibling { queue.push_front(succ); }
                    else { queue.push_back(succ); }
                }
            }
        }
        debug_assert_eq!(
            order.len(), net_num,
            "Topology sort failed: cycle or inconsistent output_topology"
        );
        order
    }

    pub fn rev_topology_nets(&self) -> Vec<NetId> {
        let net_num = self.nets.len();
        let mut rev_indegree: Vec<NetId> = vec![0; net_num];

        let mut rev_adj: Vec<Vec<NetId>> = vec![Vec::<NetId>::new(); net_num];

        for node in self.nodes() {
            for out in node.outputs() {
                for inp in node.inputs() {
                    let inp_net = inp.net();
                    rev_adj[*out].push(inp_net);
                    rev_indegree[inp_net] += 1;
                }
            }
        }

        for node in self.nodes() {
            let outputs = node.outputs();
            let topo = node.gate().output_topology();

            for w in topo.windows(2) {
                let from_orig = outputs[w[0]];
                let to_orig = outputs[w[1]];

                rev_adj[to_orig].push(from_orig);
                rev_indegree[from_orig] += 1;
            }
        }

        let mut queue = VecDeque::new();
        for net in 0..net_num {
            if rev_indegree[net] == 0 {
                queue.push_back(net);
            }
        }

        let mut order = Vec::with_capacity(net_num);
        while let Some(n) = queue.pop_front() {
            order.push(n);
            let driver_n = self.nets_at(n).driver();
            
            for &succ in &rev_adj[n] {
                rev_indegree[succ] -= 1;
                if rev_indegree[succ] == 0 {
                    let is_sibling = driver_n.is_some()
                        && driver_n == self.nets_at(succ).driver();

                    if is_sibling {
                        queue.push_front(succ);
                    } else {
                        queue.push_back(succ);
                    }
                }
            }
        }

        assert_eq!(
            order.len(), net_num,
            "Reverse topology sort failed: cycle or inconsistent output_topology"
        );
        order
    }


    pub fn bfs_cone<F>(&self, start: NetId, backward: bool, mut visit: F) 
        where F: FnMut(NetId) -> bool {
        let mut queue = VecDeque::new();
        let mut seen = vec![false; self.nets().len()];

        queue.push_back(start);
        seen[start] = true;

        while let Some(net) = queue.pop_front() {
            if !visit(net) {
                continue;
            }

            if backward {
                if let Some(node) = self.nets_at(net).driver() {
                    for lit in self.nodes_at(node).inputs() {
                        let n = lit.net();
                        if !seen[n] {
                            seen[n] = true;
                            queue.push_back(n);
                        }
                    }
                }
            } else {
                for &node in self.nets_at(net).loads() {
                    for &out in self.nodes_at(node).outputs() {
                        if !seen[out] {
                            seen[out] = true;
                            queue.push_back(out);
                        }
                    }
                }
            }
        }
    }

    // builder
    pub fn empty() -> Self {
        Self {
            nodes: Vec::new(),
            nets: vec![Net::empty()],
            inputs: Vec::new(),
            outputs: Vec::new(),
        }
    }

    pub fn add_input(&mut self) -> NetId {
        let id = self.nets.len();
        self.nets.push(Net::empty());
        self.inputs.push(id);
        id
    }

    pub fn add_gate(&mut self, gate: Gate, inputs: Vec<NetLit>) -> Vec<NetId> {
        let node_id = self.nodes.len();
        
        let mut output_nets = Vec::with_capacity(gate.n_outputs());
        for _ in 0..gate.n_outputs() {
            let net_id = self.nets.len();
            self.nets.push(Net::empty());
            self.nets[net_id].set_driver(node_id);
            output_nets.push(net_id);
        }

        for input in &inputs {
            self.nets[input.net()].add_load(node_id);
        }

        let node = Node::new(
            None,
            gate,
            inputs,
            output_nets.clone()
        );
        self.nodes.push(node);

        output_nets
    }

    pub fn set_output(&mut self, net: NetId, negative: bool) {
        if !self.outputs.iter().any(|o| o.net() == net && o.negative() == negative) {
            self.outputs.push(NetLit::new(net, negative));
        }
    }

}
