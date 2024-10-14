pub struct Statistics {
    hits: usize,
    misses: usize,
    current_size: usize,
}

impl Statistics {
    pub fn new() -> Self {
        Statistics {
            hits: 0,
            misses: 0,
            current_size: 0,
        }
    }

    pub fn hit(&mut self) {
        self.hits += 1;
    }

    pub fn miss(&mut self) {
        self.misses += 1;
    }

    pub fn update_size(&mut self, size: usize) {
        self.current_size = size;
    }

    #[allow(dead_code)]
    pub fn get_hits(&self) -> usize {
        self.hits
    }

    #[allow(dead_code)]
    pub fn get_misses(&self) -> usize {
        self.misses
    }

    #[allow(dead_code)]
    pub fn get_current_size(&self) -> usize {
        self.current_size
    }
}