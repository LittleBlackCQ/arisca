use std::fmt;

#[derive(Default)]
pub struct ReductionStats {
    pub max_size: usize,
}

impl ReductionStats {
    pub fn update_size(&mut self, size: usize) {
        self.max_size = self.max_size.max(size);
    }
}

impl fmt::Debug for ReductionStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Execution Summary:")?;

        let mut print_line = |key: &str, val: String| {
            writeln!(f, "    - {:.<25} {}", key, val)
        };

        print_line("Max Poly Size",    self.max_size.to_string())?;
        Ok(())
    }
}
