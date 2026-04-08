use std::collections::HashMap;
use serde::Serialize;

use crate::json::ToJson;
use super::{VarId};

#[derive(Serialize)]
pub struct ReductionMeta {
    pub var_map: HashMap<usize, i32>,
    pub global_seq: Vec<i32>,
    pub poly_sizes: Vec<usize>,
}

impl ReductionMeta {
    pub fn new(vars: &[VarId], global_seq: &[VarId], poly_sizes: &[usize]) -> Self {
        let var_map = vars.iter().enumerate().map(|(id, &v)| (id, v)).collect();
        Self {
            var_map,
            global_seq: global_seq.to_vec(),
            poly_sizes: poly_sizes.to_vec(),
        }
    }
}
impl ToJson for ReductionMeta {
    fn to_json_string(&self) -> String {
        serde_json::to_string(self).expect("Failed to serialize ReductionMeta")
    }
}