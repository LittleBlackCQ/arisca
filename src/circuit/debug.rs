use super::*;
use std::fmt;
use std::fs;

impl fmt::Debug for NetLit { 
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.net(), if self.negative() {"'"} else {""})
    }
}

impl fmt::Debug for Node { 
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(type: {:?}, inputs: {:?}, outputs: {:?}, name: {:?})", self.gate().name(), self.inputs(), self.outputs(), self.name())
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

impl Circuit {
    pub fn to_dot(&self, filename: &str) {
        let mut dot = String::new();
        
        dot.push_str("digraph G {\n");
        dot.push_str("    rankdir=\"BT\";\n");
        dot.push_str("    node [style=\"filled\"];\n");

        let mut net_levels = vec![0; self.nets.len()];
        let mut node_levels = vec![0; self.nodes.len()];

        for &net_id in &self.topo_order {
            if let Some(driver_id) = self.nets[net_id].driver() {
                net_levels[net_id] = node_levels[driver_id];
            } else {
                net_levels[net_id] = 0;
            }
            for &load_id in self.nets[net_id].loads() {
                node_levels[load_id] = node_levels[load_id].max(net_levels[net_id] + 1);
            }
        }

        for (i, net_id) in self.inputs.iter().enumerate() {
            let label = format!("pi_{}", i);
            dot.push_str(&format!("    net_{} [label=\"{}\", fillcolor=\"gray\", shape=\"triangle\"];\n", net_id, label));
        }

        for (i, node) in self.nodes.iter().enumerate() {
            let label = if let Some(name) = node.name() { name.to_string() } else { format!("{}_{}", node.gate().name(), i) };
            dot.push_str(&format!("    node_{} [label=\"{}\", fillcolor=\"{}\", shape=\"ellipse\"];\n", i, label, node.gate().color()));
        }

        for i in 0..self.outputs().len() {
            dot.push_str(&format!("    out_{} [label=\"po_{}\", fillcolor=\"gray\", shape=\"invtriangle\"];\n", i, i));
        }

        for (node_idx, node) in self.nodes.iter().enumerate() {
            for input_lit in node.inputs() {
                let src_net = input_lit.net();
                let style = if input_lit.negative() { "dashed" } else { "solid" };
                
                if let Some(driver_idx) = self.nets[src_net].driver() {
                    // Logic added here for multi-output labeling
                    let driver_node = &self.nodes[driver_idx];
                    let label_attr = if driver_node.outputs().len() > 1 {
                        let out_idx = driver_node.outputs().iter().position(|&n| n == src_net).unwrap_or(0);
                        format!("label=\"{}\", ", out_idx)
                    } else {
                        String::new()
                    };
                    dot.push_str(&format!("    node_{} -> node_{} [{}style=\"{}\"];\n", driver_idx, node_idx, label_attr, style));
                } else if self.inputs.contains(&src_net) {
                    dot.push_str(&format!("    net_{} -> node_{} [style=\"{}\"];\n", src_net, node_idx, style));
                }
            }
        }

        for (out_idx, out_lit) in self.outputs.iter().enumerate() {
            let src_net = out_lit.net();
            let style = if out_lit.negative() { "dashed" } else { "solid" };
            if let Some(driver_idx) = self.nets[src_net].driver() {
                // Logic added here for multi-output labeling to primary outputs
                let driver_node = &self.nodes[driver_idx];
                let label_attr = if driver_node.outputs().len() > 1 {
                    let out_idx = driver_node.outputs().iter().position(|&n| n == src_net).unwrap_or(0);
                    format!("label=\"{}\", ", out_idx)
                } else {
                    String::new()
                };
                dot.push_str(&format!("    node_{} -> out_{} [{}style=\"{}\"];\n", driver_idx, out_idx, label_attr, style));
            } else if self.inputs.contains(&src_net) {
                dot.push_str(&format!("    net_{} -> out_{} [style=\"{}\"];\n", src_net, out_idx, style));
            }
        }

        let max_level = *node_levels.iter().max().unwrap_or(&0);
        for l in 0..=max_level {
            let mut same_rank_nodes = Vec::new();
            for (idx, &lvl) in node_levels.iter().enumerate() {
                if lvl == l { same_rank_nodes.push(format!("node_{}", idx)); }
            }
            if l == 0 {
                for &net_id in &self.inputs { same_rank_nodes.push(format!("net_{}", net_id)); }
            }
            if !same_rank_nodes.is_empty() {
                dot.push_str(&format!("    {{ rank=same; {}; }}\n", same_rank_nodes.join("; ")));
            }
        }

        dot.push_str("}\n");

        fs::write(filename, dot).expect(&format!("Circuit cannot write to file {} as a dot file!", filename));
    }
}
