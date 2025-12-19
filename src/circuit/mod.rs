pub mod basics;
pub mod gate;
pub mod debug;
pub mod extractor;
pub mod cut;
pub mod sim;

use crate::circuit::basics::{Node, Net, NetId, NodeId, NetLit};
use crate::circuit::gate::Gate;
use std::collections::{VecDeque, HashMap};

pub struct Circuit {
    nodes: Vec<Node>,
    nets: Vec<Net>,
    inputs: Vec<NetId>,
    outputs: Vec<NetLit>,

    topo_order: Vec<NetId>,
}

impl Circuit {
    pub fn new(nodes: Vec<Node>, nets: Vec<Net>, inputs: Vec<NetId>, outputs: Vec<NetLit>) -> Self {
        let mut circuit = Self { 
            nodes, 
            nets, 
            inputs, 
            outputs, 
            topo_order: Vec::new() 
        };
        circuit.topo_order = circuit.compute_topology();
        circuit
    }

    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    pub fn nets(&self) -> &[Net] {
        &self.nets
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

    fn compute_topology(&self) -> Vec<NetId> {
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
            for &succ in &adj[n] {
                indegree[succ] -= 1;
                if indegree[succ] == 0 {
                    queue.push_back(succ);
                }
            }
        }
        assert_eq!(
            order.len(), net_num,
            "Topology sort failed: cycle or inconsistent output_topology"
        );
        order
    }

    pub fn topology_order(&self) -> Vec<NetId> {
       self.topo_order.clone()
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
                if let Some(node) = self.nets()[net].driver() {
                    for lit in self.nodes()[node].inputs() {
                        let n = lit.net();
                        if !seen[n] {
                            seen[n] = true;
                            queue.push_back(n);
                        }
                    }
                }
            } else {
                for &node in self.nets()[net].loads() {
                    for &out in self.nodes()[node].outputs() {
                        if !seen[out] {
                            seen[out] = true;
                            queue.push_back(out);
                        }
                    }
                }
            }
        }
    }

    pub fn subcircuit(
        &self,
        nets: &[NetId],
        nodes: &[NodeId],
        inputs: &[NetId],
        outputs: &[NetLit],
    ) -> Circuit {

        let mut net_map = HashMap::new();
        for &n in nets {
            net_map.insert(n, net_map.len());
        }

        let mut sub_nodes = Vec::with_capacity(nodes.len());
        for &nid in nodes {
            let n = &self.nodes()[nid];
            let ins = n.inputs().iter()
                .map(|l| NetLit::new(net_map[&l.net()], l.negative()))
                .collect();
            let outs = n.outputs().iter()
                .map(|o| net_map[o])
                .collect();

            sub_nodes.push(Node::new(
                n.name().clone(),
                n.gate().clone(),
                ins,
                outs,
            ));
        }

        let mut sub_nets = Vec::with_capacity(net_map.len());
        for (&old, _) in &net_map {
            sub_nets.push(Net::new(
                self.nets()[old].name().clone(),
                None,
                Vec::new(),
            ));
        }

        for (nid, node) in sub_nodes.iter().enumerate() {
            for &o in node.outputs() {
                sub_nets[o].set_driver(nid);
            }
            for inp in node.inputs() {
                sub_nets[inp.net()].add_load(nid);
            }
        }

        let sub_inputs = inputs.iter().map(|n| net_map[n]).collect();
        let sub_outputs = outputs.iter()
            .map(|l| NetLit::new(net_map[&l.net()], l.negative()))
            .collect();

        Circuit::new(sub_nodes, sub_nets, sub_inputs, sub_outputs)
    }
}
