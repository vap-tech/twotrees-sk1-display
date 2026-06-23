use std::collections::{HashMap, VecDeque};

use crate::hmi::command::HmiCommand;
use crate::thumbnail::{ThumbnailKey, ThumbnailResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThumbnailState {
    Preparing,
    Ready(Vec<HmiCommand>),
    Failed(String),
}

#[derive(Debug, Default)]
pub struct ThumbnailCache {
    entries: HashMap<ThumbnailKey, ThumbnailState>,
    ready_order: VecDeque<ThumbnailKey>,
    max_ready_entries: usize,
}

impl ThumbnailCache {
    pub fn new() -> Self {
        Self::with_max_ready_entries(64)
    }

    pub fn with_max_ready_entries(max_ready_entries: usize) -> Self {
        Self {
            entries: HashMap::new(),
            ready_order: VecDeque::new(),
            max_ready_entries,
        }
    }

    pub fn get(&self, key: &ThumbnailKey) -> Option<&ThumbnailState> {
        self.entries.get(key)
    }

    pub fn mark_preparing(&mut self, key: ThumbnailKey) {
        self.entries.insert(key, ThumbnailState::Preparing);
    }

    pub fn apply_result(&mut self, result: ThumbnailResult) {
        let key = result.key;
        let state = match result.result {
            Ok(commands) => {
                self.remember_ready_key(key.clone());
                ThumbnailState::Ready(commands)
            }
            Err(error) => ThumbnailState::Failed(error),
        };

        self.entries.insert(key, state);
        self.evict_old_ready_entries();
    }

    fn remember_ready_key(&mut self, key: ThumbnailKey) {
        self.ready_order.retain(|existing| existing != &key);
        self.ready_order.push_back(key);
    }

    fn evict_old_ready_entries(&mut self) {
        while self.ready_order.len() > self.max_ready_entries {
            let Some(key) = self.ready_order.pop_front() else {
                return;
            };

            if matches!(self.entries.get(&key), Some(ThumbnailState::Ready(_))) {
                self.entries.remove(&key);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::thumbnail::ThumbnailTarget;

    #[test]
    fn cache_stores_ready_result() {
        let mut cache = ThumbnailCache::new();
        let key = ThumbnailKey {
            file_path: "cube.gcode".to_string(),
            target: ThumbnailTarget::PrintPage,
            width: 155,
            height: 155,
            encoder_version: 1,
        };

        cache.apply_result(ThumbnailResult {
            key: key.clone(),
            result: Ok(vec![HmiCommand::raw("cp0.close()")]),
        });

        assert!(matches!(cache.get(&key), Some(ThumbnailState::Ready(_))));
    }

    #[test]
    fn cache_evicts_old_ready_entries() {
        let mut cache = ThumbnailCache::with_max_ready_entries(1);
        let first = ThumbnailKey::print("first.gcode");
        let second = ThumbnailKey::print("second.gcode");

        cache.apply_result(ThumbnailResult {
            key: first.clone(),
            result: Ok(vec![HmiCommand::raw("cp0.write(\"1\")")]),
        });
        cache.apply_result(ThumbnailResult {
            key: second.clone(),
            result: Ok(vec![HmiCommand::raw("cp0.write(\"2\")")]),
        });

        assert!(cache.get(&first).is_none());
        assert!(matches!(cache.get(&second), Some(ThumbnailState::Ready(_))));
    }
}
