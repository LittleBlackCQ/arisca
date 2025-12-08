use super::{Circuit, basics::{NetLit, Node, Net}};
use std::fmt;

impl fmt::Debug for NetLit { 
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.net(), if self.negative() {"'"} else {""})
    }
}

impl fmt::Debug for Node { 
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(type: {:?}, inputs: {:?}, outputs: {:?}, name: {:?})", self.gate_type(), self.inputs(), self.outputs(), self.name())
    }
}

impl fmt::Debug for Net { 
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(driver) = self.driver() {
            write!(f, "(driver: {:?}, ", driver)?;
        } else {
            write!(f, "(driver: None, ")?;
        }
        write!(f, "loads: {:?}, name: {:?})", self.loads(), self.name())
    }
}

impl fmt::Debug for Circuit { 
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for i in 0..self.nodes.len() {
            write!(f, "Node {:?}: {:?}\n", i, self.nodes[i])?;
        }
        for i in 0..self.nets.len() {
            write!(f, "Net {:?}: {:?}\n", i, self.nets[i])?;
        }
        write!(f, "Inputs: {:?}\n", self.inputs)?;
        write!(f, "Outputs: {:?}\n", self.outputs)
    }
}
