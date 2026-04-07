use serde_json::json;
use std::collections::hash_map::DefaultHasher;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{self, Write};
use std::path::Path;

use super::*;

pub trait ToJson {
    fn to_json_string(&self) -> String;

    fn write_json(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let json = self.to_json_string();
        let mut file = File::create(path)?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }
}

impl ToJson for Circuit {
    fn to_json_string(&self) -> String {
        let mut hasher = DefaultHasher::new();
        self.nodes.len().hash(&mut hasher);
        self.nets.len().hash(&mut hasher);
        let graph_id = format!("{:016x}", hasher.finish());

        let payload = json!({
            "graph_id": graph_id,
            "num_nodes": self.nodes.len(),
            "num_nets": self.nets.len(),
            "hypergraph": {
                "nodes": self.nodes.iter().map(|n| {
                    json!({
                        "gate": n.gate().name(),
                        "inputs": n.inputs().iter().map(|lit| (lit.net(), lit.negative() as u8)).collect::<Vec<_>>(),
                        "outputs": n.outputs(),
                    })
                }).collect::<Vec<_>>(),
                "nets": self.nets.iter().map(|n| {
                    json!({
                        "driver": n.driver(),
                        "loads": n.loads(),
                    })
                }).collect::<Vec<_>>(),
            }
        });

        serde_json::to_string(&payload).expect("Serialization failed")
    }
}