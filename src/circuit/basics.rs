use super::gate::Gate;


pub type NodeId = usize;
pub type NetId = usize;

pub struct NetLit {
    net: NetId,
    negative: bool,
}

impl NetLit {
    pub fn new(net: NetId, negative: bool) -> Self {
        NetLit {
            net,
            negative,
        }
    }

    pub fn positive(net: NetId) -> Self {
        NetLit::new(net, false)
    }

    pub fn negative(&self) -> bool {
        self.negative
    }

    pub fn net(&self) -> NetId {
        self.net
    }
}

pub struct Node {
    name: Option<Box<str>>,
    gate: Gate,
    inputs: Vec<NetLit>,
    outputs: Vec<NetId>,
}

impl Node {
    pub fn new(name: Option<Box<str>>, gate: Gate, inputs: Vec<NetLit>, outputs: Vec<NetId>) -> Self {
        Node {
            name,
            gate,
            inputs,
            outputs,
        }
    }
    pub fn name(&self) -> &str {
        self.name.as_deref().unwrap_or("")
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
}

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

    pub fn name(&self) -> &str {
        self.name.as_deref().unwrap_or("")
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
