use std::collections::{HashMap, VecDeque};

use crate::domain::runtime::{ProcessLogPayload, ProcessRuntimeId};

pub trait LogStore {
    fn append(&mut self, payload: ProcessLogPayload);
    fn list(&self, runtime_id: &ProcessRuntimeId) -> Vec<ProcessLogPayload>;
    fn clear(&mut self, runtime_id: &ProcessRuntimeId);
}

#[derive(Debug, Clone)]
pub struct InMemoryLogStore {
    limit: usize,
    entries: HashMap<ProcessRuntimeId, VecDeque<ProcessLogPayload>>,
}

impl InMemoryLogStore {
    pub fn new(limit: usize) -> Self {
        Self {
            limit,
            entries: HashMap::new(),
        }
    }
}

impl Default for InMemoryLogStore {
    fn default() -> Self {
        Self::new(10_000)
    }
}

impl LogStore for InMemoryLogStore {
    fn append(&mut self, payload: ProcessLogPayload) {
        let runtime_id = payload.runtime_id.clone();
        let queue = self.entries.entry(runtime_id).or_default();
        queue.push_back(payload);
        while queue.len() > self.limit {
            queue.pop_front();
        }
    }

    fn list(&self, runtime_id: &ProcessRuntimeId) -> Vec<ProcessLogPayload> {
        self.entries
            .get(runtime_id)
            .map(|queue| queue.iter().cloned().collect())
            .unwrap_or_default()
    }

    fn clear(&mut self, runtime_id: &ProcessRuntimeId) {
        self.entries.remove(runtime_id);
    }
}
