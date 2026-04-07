pub mod basics;
pub mod gate;
pub mod debug;
pub mod extractor;
pub mod cut;
pub mod sim;
mod to_aig;
mod to_json;

pub use basics::{Node, Net, NetId, NodeId, NetLit, Cone};
pub use extractor::{AdderExtractor, GenericExtractor, XorExtractor, Xor3Extractor, MajExtractor};
pub use gate::Gate;
pub use to_json::ToJson;

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
    pub fn get_dfs_cone<F>(&self, root: NetId, is_terminal: F) -> Cone
    where F: Fn(&NetId) -> bool { 
        use std::collections::HashSet;
        let (mut nets, mut nodes, mut inputs) = (Vec::new(), Vec::new(), Vec::new());
        
        fn dfs<F>(circuit: &Circuit, net: NetId, is_terminal: &F, nets: &mut Vec<NetId>, nodes: &mut Vec<NodeId>, inputs: &mut Vec<NetId>, visited: &mut HashSet<NetId>)
        where F: Fn(&NetId) -> bool {
            if visited.contains(&net) { return; }
            visited.insert(net);
            if is_terminal(&net) { inputs.push(net); return; }
            if let Some(driver) = circuit.nets_at(net).driver() {
                for input in circuit.nodes_at(driver).inputs() {
                    dfs(circuit, input.net(), is_terminal, nets, nodes, inputs, visited);
                }
                nodes.push(driver);
            }
            nets.push(net);
        }
        dfs(self, root, &is_terminal, &mut nets, &mut nodes, &mut inputs, &mut HashSet::new());
        Cone { root, inputs, nets, nodes }
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
