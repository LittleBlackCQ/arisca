use super::*;
use std::fmt;
use std::path::Path;
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
            write!(f, "Node {:?}: {:?}\n", i, self.nodes_at(i))?;
        }
        for i in 0..self.nets.len() {
            write!(f, "Net {:?}: {:?}\n", i, self.nets_at(i))?;
        }
        write!(f, "Inputs: {:?}\n", self.inputs)?;
        write!(f, "Outputs: {:?}\n", self.outputs)
    }
}

fn get_gate_color(g: &Gate) -> &'static str {
    match g {
        Gate::And => "lightcoral",
        Gate::Or | Gate::FullAdder => "lightskyblue",
        Gate::Xor | Gate::HalfAdder | Gate::Xor3 => "palegreen2",
        Gate::Maj => "gold",
    }
}

impl Circuit {
   pub fn to_dot(&self, path: impl AsRef<Path>, cone: Option<&Cone>) {
        let mut dot = String::new();

        let (valid_nodes, valid_cone_inputs, cone_root) = if let Some(c) = cone {
            (
                Some(c.nodes.iter().cloned().collect::<std::collections::HashSet<_>>()),
                Some(c.inputs.iter().cloned().collect::<std::collections::HashSet<_>>()),
                Some(c.root)
            )
        } else {
            (None, None, None)
        };

        let is_node_visible = |node_id: usize| -> bool {
            valid_nodes.as_ref().map_or(true, |set| set.contains(&node_id))
        };

        dot.push_str("digraph G {\n");
        dot.push_str("    rankdir=\"BT\";\n");
        dot.push_str("    node [style=\"filled\"];\n");

        let mut net_levels = vec![0; self.nets.len()];
        let mut node_levels = vec![0; self.nodes.len()];

        for net_id in 0..self.nets().len() {
            if let Some(driver_id) = self.nets_at(net_id).driver() {
                net_levels[net_id] = node_levels[driver_id];
            } else {
                net_levels[net_id] = 0;
            }
            for &load_id in self.nets_at(net_id).loads() {
                node_levels[load_id] = node_levels[load_id].max(net_levels[net_id] + 1);
            }
        }

        if let Some(cone_inputs) = &valid_cone_inputs {
            for &net_id in cone_inputs.iter() {
                let label = format!("cone_in_{}", net_id);
                dot.push_str(&format!("    net_{} [label=\"{}\", fillcolor=\"gray\", shape=\"triangle\"];\n", net_id, label));
            }
        } else {
            for (i, &net_id) in self.inputs.iter().enumerate() {
                let label = format!("pi_{}", i);
                dot.push_str(&format!("    net_{} [label=\"{}\", fillcolor=\"gray\", shape=\"triangle\"];\n", net_id, label));
            }
        }

        for (i, node) in self.nodes.iter().enumerate() {
            if !is_node_visible(i) { continue; }
            let label = if let Some(name) = node.name() { name.to_string() } else { format!("{}_{}", node.gate().name(), i) };
            dot.push_str(&format!("    node_{} [label=\"{}\", fillcolor=\"{}\", shape=\"ellipse\"];\n", i, label, get_gate_color(node.gate())));
        }

        if let Some(root_net) = cone_root {
            dot.push_str(&format!("    out_root [label=\"root_{}\", fillcolor=\"gray\", shape=\"invtriangle\"];\n", root_net));
        } else {
            for i in 0..self.outputs().len() {
                dot.push_str(&format!("    out_{} [label=\"po_{}\", fillcolor=\"gray\", shape=\"invtriangle\"];\n", i, i));
            }
        }

        for (node_idx, node) in self.nodes.iter().enumerate() {
            if !is_node_visible(node_idx) { continue; }

            for input_lit in node.inputs() {
                let src_net = input_lit.net();
                let style = if input_lit.negative() { "dashed" } else { "solid" };

                if let Some(driver_idx) = self.nets_at(src_net).driver() {
                    if is_node_visible(driver_idx) {
                        let driver_node = self.nodes_at(driver_idx);
                        let label_attr = if driver_node.outputs().len() > 1 {
                            let out_idx = driver_node.outputs().iter().position(|&n| n == src_net).unwrap_or(0);
                            format!("label=\"{}-{}\", ", src_net, out_idx)
                        } else {
                            format!("label=\"{}\", ", src_net)
                        };
                        dot.push_str(&format!("    node_{} -> node_{} [{}style=\"{}\"];\n", driver_idx, node_idx, label_attr, style));
                    } else if valid_cone_inputs.as_ref().map_or(false, |s| s.contains(&src_net)) {
                        dot.push_str(&format!("    net_{} -> node_{} [style=\"{}\"];\n", src_net, node_idx, style));
                    }
                } else {
                    let is_global_pi = self.inputs.contains(&src_net) && cone.is_none();
                    let is_cone_input = valid_cone_inputs.as_ref().map_or(false, |s| s.contains(&src_net));

                    if is_global_pi || is_cone_input {
                        dot.push_str(&format!("    net_{} -> node_{} [style=\"{}\"];\n", src_net, node_idx, style));
                    }
                }
            }
        }

        if let Some(root_net) = cone_root {
            if let Some(driver_idx) = self.nets_at(root_net).driver() {
                if is_node_visible(driver_idx) {
                    let driver_node = self.nodes_at(driver_idx);
                    let label_attr = if driver_node.outputs().len() > 1 {
                        let out_idx = driver_node.outputs().iter().position(|&n| n == root_net).unwrap_or(0);
                        format!("label=\"{}-{}\", ", root_net, out_idx)
                    } else {
                        format!("label=\"{}\", ", root_net)
                    };
                    dot.push_str(&format!("    node_{} -> out_root [{}style=\"solid\"];\n", driver_idx, label_attr));
                }
            }
        } else {
            for (out_idx, out_lit) in self.outputs.iter().enumerate() {
                let src_net = out_lit.net();
                let style = if out_lit.negative() { "dashed" } else { "solid" };
                if let Some(driver_idx) = self.nets_at(src_net).driver() {
                    let driver_node = self.nodes_at(driver_idx);
                    let label_attr = if driver_node.outputs().len() > 1 {
                        let out_idx = driver_node.outputs().iter().position(|&n| n == src_net).unwrap_or(0);
                        format!("label=\"{}-{}\", ", src_net, out_idx)
                    } else {
                        format!("label=\"{}\", ", src_net)
                    };
                    dot.push_str(&format!("    node_{} -> out_{} [{}style=\"{}\"];\n", driver_idx, out_idx, label_attr, style));
                } else if self.inputs.contains(&src_net) {
                    dot.push_str(&format!("    net_{} -> out_{} [style=\"{}\"];\n", src_net, out_idx, style));
                }
            }
        }

        let max_level = *node_levels.iter().max().unwrap_or(&0);
        for l in 0..=max_level {
            let mut same_rank_nodes = Vec::new();
            for (idx, &lvl) in node_levels.iter().enumerate() {
                if lvl == l && is_node_visible(idx) {
                    same_rank_nodes.push(format!("node_{}", idx));
                }
            }
            if l == 0 {
                if let Some(cone_inputs) = &valid_cone_inputs {
                    for &net_id in cone_inputs { same_rank_nodes.push(format!("net_{}", net_id)); }
                } else {
                    for &net_id in &self.inputs { same_rank_nodes.push(format!("net_{}", net_id)); }
                }
            }
            if !same_rank_nodes.is_empty() {
                dot.push_str(&format!("    {{ rank=same; {}; }}\n", same_rank_nodes.join("; ")));
            }
        }

        if cone.is_some() {
            dot.push_str("    { rank=same; out_root; }\n");
        } else {
            let mut po_nodes = Vec::new();
            for i in 0..self.outputs().len() {
                po_nodes.push(format!("out_{}", i));
            }
            if !po_nodes.is_empty() {
                dot.push_str(&format!("    {{ rank=same; {}; }}\n", po_nodes.join("; ")));
            }
        }

        dot.push_str("}\n");
        fs::write(path.as_ref(), dot).expect(&format!("Circuit cannot write to file {} as a dot file!", path.as_ref().display()));
    }
}
