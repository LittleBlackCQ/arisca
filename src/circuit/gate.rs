#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Gate {
    And,
    Or,
    Xor,
    HalfAdder,
    FullAdder,
}

impl Gate {
    pub fn n_inputs(self) -> usize {
        match self {
            Gate::And => 2,
            Gate::Or => 2,
            Gate::Xor => 2,
            Gate::HalfAdder => 2,
            Gate::FullAdder => 3,
        }
    }

    pub fn n_outputs(self) -> usize {
        match self {
            Gate::And => 1,
            Gate::Or => 1,
            Gate::Xor => 1,
            Gate::HalfAdder => 2,   // sum, carry
            Gate::FullAdder => 2,   // sum, carry
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Gate::And => "and",
            Gate::Or => "or",
            Gate::Xor => "xor",
            Gate::HalfAdder => "half_adder",
            Gate::FullAdder => "full_adder",
        }
    }

    pub fn logic(self, inputs: &[bool]) -> Vec<bool> {
        assert_eq!(inputs.len(), self.n_inputs(), "Wrong number of inputs for {:?} in logic.", self.name());
        let mut outputs = vec![false; self.n_outputs()];
        match self {
            Gate::And => {
                outputs[0] = inputs[0] & inputs[1];
            }
            Gate::Or => {
                outputs[0] = inputs[0] | inputs[1];
            }
            Gate::Xor => {
                outputs[0] = inputs[0] ^ inputs[1];
            }
            Gate::HalfAdder => {
                outputs[0] = inputs[0] ^ inputs[1]; // sum
                outputs[1] = inputs[0] & inputs[1]; // carry
            }
            Gate::FullAdder => {
                outputs[0] = inputs[0] ^ inputs[1] ^ inputs[2]; // sum
                outputs[1] = (inputs[0] & inputs[1]) | (inputs[1] & inputs[2]) | (inputs[0] & inputs[2]) // carry
            }
        }
        assert_eq!(outputs.len(), self.n_outputs(), "Wrong number of outputs for {:?} in logic.", self.name());
        outputs
    }
}

