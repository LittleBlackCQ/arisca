pub mod basics;
pub mod gate;
pub mod debug;
pub mod extractor;
pub mod cut;
pub mod sim;

pub use crate::circuit::basics::{Node, Net, NetId, NodeId, NetLit, Cone};
pub use crate::circuit::extractor::AdderExtractor;
pub use crate::circuit::gate::Gate;

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
        self.nodes().iter().any(|n| n.outputs().len() > 1)
    }

    pub fn is_topo_sorted(&self) -> bool {
        for node in &self.nodes {
            let inputs = node.inputs();
            let outputs = node.outputs();

            if let Some(max_input_id) = inputs.iter().map(|lit| lit.net()).max() {
                if let Some(min_output_id) = outputs.iter().min() {
                    if max_input_id >= *min_output_id {
                        return false;
                    }
                }
            }
        }
        true
    }

    // cone
    pub fn get_levelized_cone<F>(&self, root: NetId, is_terminal: F) -> Cone
    where F: Fn(&NetId) -> bool {
        use std::collections::{HashMap, VecDeque};
        let mut degrees = HashMap::new();
        let mut stack = vec![root];
        while let Some(net) = stack.pop() {
            let count = degrees.entry(net).or_insert(0);
            *count += 1;
            if *count == 1 && !is_terminal(&net) {
                if let Some(driver) = self.nets_at(net).driver() {
                    stack.extend(self.nodes_at(driver).inputs().iter().map(|n| n.net()));
                }
            }
        }

        let mut cone = Cone { root, inputs: Vec::new(), nets: Vec::new(), nodes: Vec::new() };
        let mut queue = VecDeque::from([root]);

        degrees.remove(&root);

        while let Some(net) = queue.pop_front() {
            if is_terminal(&net) {
                cone.inputs.push(net);
                continue;
            }
            cone.nets.push(net);
            if let Some(driver) = self.nets_at(net).driver() {
                cone.nodes.push(driver);
                for input in self.nodes_at(driver).inputs() {
                    let input_net = input.net();
                    if let Some(c) = degrees.get_mut(&input_net) {
                        *c -= 1;
                        if *c == 0 {
                            queue.push_back(input_net);
                        }
                    }
                }
            }
        }
        cone.nets.reverse();
        cone.nodes.reverse();
        cone
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
