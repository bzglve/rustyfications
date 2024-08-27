use std::sync::atomic::{AtomicU32, Ordering};

static ID: AtomicU32 = AtomicU32::new(0);

pub struct Id(u32);

impl Id {
    pub fn new() -> Self {
        Self(Self::current_glob())
    }

    pub fn current_glob() -> u32 {
        ID.load(Ordering::Relaxed)
    }

    pub fn bump_glob() -> u32 {
        ID.fetch_add(1, Ordering::Relaxed) + 1
    }

    pub fn bump(&mut self) -> u32 {
        self.0 = Self::bump_glob();
        self.0
    }
}

impl Default for Id {
    fn default() -> Self {
        Self::new()
    }
}
