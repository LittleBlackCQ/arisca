use std::fmt;
use std::time::{Duration, Instant};

pub struct ReductionStats {
    pub max_size: usize,
    pub start_time: Instant,
    pub last_tick: Instant,
}

impl ReductionStats {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            last_tick: Instant::now(),
            max_size: 0,
        }
    }

    pub fn update_size(&mut self, size: usize) {
        self.max_size = self.max_size.max(size);
    }

    pub fn tick(&mut self) -> Duration {
        let now = Instant::now();
        let duration = now.duration_since(self.last_tick);
        self.last_tick = now;
        duration
    }

    pub fn total_elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }
}

impl fmt::Debug for ReductionStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Execution Summary:")?;

        let mut print_line = |key: &str, val: String| writeln!(f, "    - {:.<25} {}", key, val);

        print_line("Max Poly Size", self.max_size.to_string())?;
        print_line("Total Time", format!("{:?}", self.total_elapsed()))?;
        Ok(())
    }
}
