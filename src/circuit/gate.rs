use std::any::Any;

pub trait Gate: Any {
    fn name(&self) -> &'static str;
    fn n_inputs(&self) -> usize;
    fn n_outputs(&self) -> usize;
    fn logic(&self, inputs: &[bool], outputs: &mut [bool]);
    fn eval(&self, inputs: &[bool], outputs: &mut [bool]) {
        assert_eq!(inputs.len(), self.n_inputs());
        assert_eq!(outputs.len(), self.n_outputs());
        self.logic(inputs, outputs)
    }
}


pub struct AndGate;
impl Gate for AndGate {
    fn name(&self) -> &'static str { "and" }
    fn n_inputs(&self) -> usize { 2 }
    fn n_outputs(&self) -> usize { 1 }
    fn logic(&self, inputs: &[bool], outputs: &mut [bool]) {
        outputs[0] = inputs[0] & inputs[1];
    }
}


pub struct OrGate;
impl Gate for OrGate {
    fn name(&self) -> &'static str { "or" }
    fn n_inputs(&self) -> usize { 2 }
    fn n_outputs(&self) -> usize { 1 }
    fn logic(&self, inputs: &[bool], outputs: &mut [bool]) {
        outputs[0] = inputs[0] | inputs[1];
    }
}


pub struct XorGate;
impl Gate for XorGate {
    fn name(&self) -> &'static str { "xor" }
    fn n_inputs(&self) -> usize { 2 }
    fn n_outputs(&self) -> usize { 1 }
    fn logic(&self, inputs: &[bool], outputs: &mut [bool]) {
        outputs[0] = inputs[0] ^ inputs[1];
    }
}


pub struct HalfAdderGate;
impl Gate for HalfAdderGate {
    fn name(&self) -> &'static str { "half_adder" }
    fn n_inputs(&self) -> usize { 2 }
    fn n_outputs(&self) -> usize { 2 }
    fn logic(&self, inputs: &[bool], outputs: &mut [bool]) {
        outputs[0] = inputs[0] ^ inputs[1]; // sum
        outputs[1] = inputs[0] & inputs[1]; // carry
    }
}


pub struct FullAdderGate;
impl Gate for FullAdderGate {
    fn name(&self) -> &'static str { "full_adder" }
    fn n_inputs(&self) -> usize { 3 }
    fn n_outputs(&self) -> usize { 2 }
    fn logic(&self, inputs: &[bool], outputs: &mut [bool]) {
        outputs[0] = inputs[0] ^ inputs[1] ^ inputs[2]; // sum
        outputs[1] = (inputs[0] & inputs[1]) | (inputs[1] & inputs[2]) | (inputs[0] & inputs[2]); // carry
    }
}
