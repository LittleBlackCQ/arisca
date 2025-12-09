pub mod basics;
pub mod gate;
pub mod debug;

use crate::circuit::basics::{Node, Net, NetId, NetLit};

pub struct Circuit {
    nodes: Vec<Node>,
    nets: Vec<Net>,
    inputs: Vec<NetId>,
    outputs: Vec<NetLit>,
}

impl Circuit {
    pub fn new(nodes: Vec<Node>, nets: Vec<Net>, inputs: Vec<NetId>, outputs: Vec<NetLit>) -> Self {
        Self { nodes, nets, inputs, outputs }
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
}