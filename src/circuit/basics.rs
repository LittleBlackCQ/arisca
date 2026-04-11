use super::gate::Gate;

pub type NodeId = usize;
pub type NetId = usize;

#[derive(Clone, Copy, Hash)]
pub struct NetLit {
    net: NetId,
    negative: bool,
}

impl NetLit {
    pub fn new(net: NetId, negative: bool) -> Self {
        NetLit { net, negative }
    }

    pub fn from_positive(net: NetId) -> Self {
        NetLit::new(net, false)
    }

    pub fn from_negative(net: NetId) -> Self {
        NetLit::new(net, true)
    }

    pub fn negative(&self) -> bool {
        self.negative
    }

    pub fn net(&self) -> NetId {
        self.net
    }
}

#[derive(Clone, Hash)]
pub struct Node {
    name: Option<Box<str>>,
    gate: Gate,
    inputs: Vec<NetLit>,
    outputs: Vec<NetId>,
}

impl Node {
    pub fn new(
        name: Option<Box<str>>,
        gate: Gate,
        inputs: Vec<NetLit>,
        outputs: Vec<NetId>,
    ) -> Self {
        Node {
            name,
            gate,
            inputs,
            outputs,
        }
    }
    pub fn name(&self) -> &Option<Box<str>> {
        &self.name
    }

    pub fn gate(&self) -> &Gate {
        &self.gate
    }

    pub fn inputs(&self) -> &[NetLit] {
        &self.inputs
    }

    pub fn outputs(&self) -> &[NetId] {
        &self.outputs
    }

    pub fn is_multioutput(&self) -> bool {
        self.outputs.len() > 1
    }
}

#[derive(Clone, Hash)]
pub struct Net {
    name: Option<Box<str>>,
    driver: Option<NodeId>,
    loads: Vec<NodeId>,
}

impl Net {
    pub fn new(name: Option<Box<str>>, driver: Option<NodeId>, loads: Vec<NodeId>) -> Self {
        Net {
            name,
            driver,
            loads,
        }
    }

    pub fn empty() -> Self {
        Net::new(None, None, vec![])
    }

    pub fn name(&self) -> &Option<Box<str>> {
        &self.name
    }

    pub fn set_driver(&mut self, driver: NodeId) {
        self.driver = Some(driver);
    }

    pub fn add_load(&mut self, load: NodeId) {
        self.loads.push(load);
    }

    pub fn driver(&self) -> Option<NodeId> {
        self.driver
    }

    pub fn loads(&self) -> &[NodeId] {
        &self.loads
    }
}

#[derive(Clone, Hash, Debug)]
pub struct Cone {
    pub root: NetId,
    pub inputs: Vec<NetId>,
    pub nets: Vec<NetId>,
    pub nodes: Vec<NodeId>,
}
