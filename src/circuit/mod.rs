pub mod basics;
pub mod gate;
pub mod debug;

use crate::circuit::basics::{Node, Net, NetId, NetLit};

pub struct Circuit {
    pub nodes: Vec<Node>,
    pub nets: Vec<Net>,
    pub inputs: Vec<NetId>,
    pub outputs: Vec<NetLit>,
}

impl Circuit {
    pub fn new(nodes: Vec<Node>, nets: Vec<Net>, inputs: Vec<NetId>, outputs: Vec<NetLit>) -> Self {
        Self { nodes, nets, inputs, outputs }
    }
}