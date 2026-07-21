use std::sync::Mutex;
use super::types::BlackboxEvent;

pub struct EventBuffer {
    capacity: usize,
    buffer: Mutex<Vec<BlackboxEvent>>,
}

impl EventBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            buffer: Mutex::new(Vec::with_capacity(100)),
        }
    }

    pub fn push(&self, event: BlackboxEvent) -> bool {
        let mut lock = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
        if lock.len() >= self.capacity {
            lock.remove(0); // FIFO eviction when full
        }
        lock.push(event);
        lock.len() >= 100 // Trigger flush if >= 100 items
    }

    pub fn drain(&self) -> Vec<BlackboxEvent> {
        let mut lock = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
        std::mem::take(&mut *lock)
    }

    pub fn len(&self) -> usize {
        let lock = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
        lock.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
