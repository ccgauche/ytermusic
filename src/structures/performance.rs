use std::time::Instant;

use log::info;
use once_cell::sync::Lazy;

pub struct Performance {
    pub initial: Instant,
}

impl Performance {
    pub fn new() -> Self {
        Self {
            initial: Instant::now(),
        }
    }

    pub fn get_ms(&self) -> u128 {
        self.initial.elapsed().as_millis()
    }

    pub fn log(&self, message: &str) {
        info!(target: "performance", "{}: {}ms", message, self.get_ms());
    }
}

pub fn guard(name: &str) -> PerformanceGuard {
    PerformanceGuard::new(name)
}

pub struct PerformanceGuard<'a> {
    name: &'a str,
    start: Performance,
}

impl<'a> PerformanceGuard<'a> {
    pub fn new(name: &'a str) -> Self {
        Self {
            name,
            start: Performance::new(),
        }
    }
}

impl<'a> Drop for PerformanceGuard<'a> {
    fn drop(&mut self) {
        self.start.log(self.name);
    }
}

#[allow(dead_code)]
pub fn mesure<T>(name: &str, f: impl FnOnce() -> T) -> T {
    let start = Instant::now();
    let t = f();
    let end = Instant::now();
    info!(target: "performance", 
        "{}: {}ms",
        name,
        end.duration_since(start).as_millis()
    );
    t
}

pub static STARTUP_TIME: Lazy<Performance> = Lazy::new(Performance::new);
