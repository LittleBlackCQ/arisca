#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Gate {
    And,
    Or,
    Xor,
    Xor3,
    Maj,
    HalfAdder,
    FullAdder,
}

impl Gate {
    pub fn n_inputs(self) -> usize {
        match self {
            Gate::And | Gate::Or | Gate::Xor | Gate::HalfAdder => 2,
            Gate::FullAdder | Gate::Xor3 | Gate::Maj => 3,
        }
    }

    pub fn n_outputs(self) -> usize {
        match self {
            Gate::And | Gate::Or | Gate::Xor | Gate::Xor3 | Gate::Maj => 1,
            Gate::HalfAdder | Gate::FullAdder => 2,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Gate::And => "and",
            Gate::Or => "or",
            Gate::Xor => "xor",
            Gate::Xor3 => "xor3",
            Gate::Maj => "maj",
            Gate::HalfAdder => "ha",
            Gate::FullAdder => "fa",
        }
    }

    pub fn output_topology(self) -> Vec<usize> {
        match self {
            Gate::HalfAdder | Gate::FullAdder => vec![1, 0],
            _ => vec![0],
        }
    }

    pub fn logic(self, inputs: &[bool]) -> Vec<bool> {
        debug_assert_eq!(inputs.len(), self.n_inputs(), "Wrong number of inputs for {:?} in logic.", self.name());
        let mut outputs = vec![false; self.n_outputs()];
        match self {
            Gate::And => outputs[0] = inputs[0] & inputs[1],
            Gate::Or => outputs[0] = inputs[0] | inputs[1],
            Gate::Xor => outputs[0] = inputs[0] ^ inputs[1],
            Gate::Xor3 => outputs[0] = inputs[0] ^ inputs[1] ^ inputs[2],
            Gate::Maj => outputs[0] = (inputs[0] & inputs[1]) | (inputs[1] & inputs[2]) | (inputs[0] & inputs[2]),
            Gate::HalfAdder => {
                outputs[0] = inputs[0] ^ inputs[1]; // sum
                outputs[1] = inputs[0] & inputs[1]; // carry
            }
            Gate::FullAdder => {
                outputs[0] = inputs[0] ^ inputs[1] ^ inputs[2]; // sum
                outputs[1] = (inputs[0] & inputs[1]) | (inputs[1] & inputs[2]) | (inputs[0] & inputs[2]) // carry
            }
        }
        outputs
    }
    
}