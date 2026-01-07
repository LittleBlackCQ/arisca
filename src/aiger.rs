use libc::{FILE, fclose, fopen};
use std::{
    ffi::{CString, c_char, c_void}, path::Path
};
use log::warn;

unsafe extern "C" {
    fn aiger_init() -> *mut c_void;
    fn aiger_read_from_file(aiger: *mut c_void, file: *mut FILE) -> *mut c_char;
}

#[repr(C)]
struct Aiger {
    maxvar: u32,
    num_inputs: u32,
    num_latches: u32,
    num_outputs: u32,
    num_ands: u32,
    num_bad: u32,
    num_constraints: u32,
    num_justice: u32,
    num_fairness: u32,

    // [0..num_inputs[
    inputs: *mut AigerSymbol,
    // [0..num_latches[
    latches: *mut AigerSymbol,
    // [0..num_outputs[
    outputs: *mut AigerSymbol,
    // [0..num_bad[
    bad: *mut AigerSymbol,
    // [0..num_constraints[
    constraints: *mut AigerSymbol,
    // [0..num_justice[
    justice: *mut AigerSymbol,
    // [0..num_fairness[
    fairness: *mut AigerSymbol,
    ands: *mut AigerAnd,
    comments: *mut *mut c_char,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct AigerSymbol {
    lit: u32,
    next: u32,
    reset: u32,
    size: u32,
    lits: *mut u32,
    name: *mut c_char,
}

#[repr(C)]
struct AigerAnd {
    lhs: u32,
    rhs0: u32,
    rhs1: u32,
}

use crate::circuit::*;

pub struct AigerParser;

impl AigerParser {
    pub fn from_aig<P: AsRef<Path>>(path: P) -> Result<Circuit, String> { 
        let path = path.as_ref();
        let file = CString::new(path.to_str().unwrap()).unwrap();
        let mode = CString::new("r").unwrap();
        let file = unsafe { fopen(file.as_ptr(), mode.as_ptr())};
        if file.is_null() {
            return Err(format!("'{}' not found.", path.display()));
        }

        let aiger = unsafe { aiger_init() };
        if !unsafe { aiger_read_from_file(aiger, file) }.is_null() {
            return Err(format!("read file '{}' failed.", path.display()));
        }
        unsafe { fclose(file) };

        let aiger = unsafe { &mut *(aiger as *mut Aiger) };

        if aiger.num_bad > 0 || aiger.num_constraints > 0 || aiger.num_justice > 0 || aiger.num_fairness > 0 || aiger.num_latches > 0 {
            warn!("aiger file contains unsupported features (bad, constraints, justice, fairness, latches).")
        }
        
        let mut circuit = Circuit::empty();

        let maxvar = aiger.maxvar as usize;
        let mut var_to_net = vec![0; maxvar + 1];

        for i in 0..aiger.num_inputs {
            let sym = unsafe { &*aiger.inputs.add(i as usize) };
            let aig_var = (sym.lit / 2) as usize;
            
            let net_id = circuit.add_input();
            var_to_net[aig_var] = net_id;
        }

        for i in 0..aiger.num_ands {
            let and = unsafe { &*aiger.ands.add(i as usize) };
            let lhs_var = (and.lhs / 2) as usize;

            let rhs0_var = (and.rhs0 / 2) as usize;
            let rhs1_var = (and.rhs1 / 2) as usize;
            let rhs0_neg = (and.rhs0 & 1) != 0;
            let rhs1_neg = (and.rhs1 & 1) != 0;

            let net0 = var_to_net[rhs0_var];
            let net1 = var_to_net[rhs1_var];

            let inputs = vec![
                NetLit::new(net0, rhs0_neg),
                NetLit::new(net1, rhs1_neg),
            ];

            let output_nets = circuit.add_gate(Gate::And, inputs);
            var_to_net[lhs_var] = output_nets[0];
        }

        for i in 0..aiger.num_outputs {
            let sym = unsafe { &*aiger.outputs.add(i as usize) };
            let var = (sym.lit / 2) as usize;
            let neg = (sym.lit & 1) != 0;
            circuit.set_output(var_to_net[var], neg);
        }

        Ok(circuit)
    }
}
