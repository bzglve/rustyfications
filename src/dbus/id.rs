use std::sync::atomic::{AtomicU32, Ordering};

static ID: AtomicU32 = AtomicU32::new(0);

pub struct Id;

impl Id {
    pub fn current_glob() -> u32 {
        ID.load(Ordering::Relaxed)
    }

    pub fn bump_glob() -> u32 {
        ID.fetch_add(1, Ordering::Relaxed) + 1
    }
}
