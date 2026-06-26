use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
// Token usage tracking: PairTokenUsage + TokenTracker.

#[derive(Debug, Default)]
pub struct PairTokenUsage {
    pub input_tokens: AtomicU64,
    pub output_tokens: AtomicU64,
}

impl PairTokenUsage {
    pub fn accumulate(&self, input: u64, output: u64) {
        self.input_tokens.fetch_add(input, Ordering::Relaxed);
        self.output_tokens.fetch_add(output, Ordering::Relaxed);
    }

    pub fn load(&self) -> (u64, u64) {
        (
            self.input_tokens.load(Ordering::Relaxed),
            self.output_tokens.load(Ordering::Relaxed),
        )
    }

    pub fn total(&self) -> u64 {
        self.input_tokens.load(Ordering::Relaxed) + self.output_tokens.load(Ordering::Relaxed)
    }

    pub fn reset(&self) {
        self.input_tokens.store(0, Ordering::Relaxed);
        self.output_tokens.store(0, Ordering::Relaxed);
    }
}

impl Clone for PairTokenUsage {
    fn clone(&self) -> Self {
        let (i, o) = self.load();
        PairTokenUsage {
            input_tokens: AtomicU64::new(i),
            output_tokens: AtomicU64::new(o),
        }
    }
}

impl Serialize for PairTokenUsage {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("PairTokenUsage", 2)?;
        s.serialize_field("input_tokens", &self.input_tokens.load(Ordering::Relaxed))?;
        s.serialize_field("output_tokens", &self.output_tokens.load(Ordering::Relaxed))?;
        s.end()
    }
}

impl<'de> Deserialize<'de> for PairTokenUsage {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Helper {
            input_tokens: u64,
            output_tokens: u64,
        }
        let h = Helper::deserialize(deserializer)?;
        Ok(PairTokenUsage {
            input_tokens: AtomicU64::new(h.input_tokens),
            output_tokens: AtomicU64::new(h.output_tokens),
        })
    }
}

#[derive(Debug, Default)]
pub struct TokenTracker {
    pub per_pair: Mutex<HashMap<String, PairTokenUsage>>,
    pub global: PairTokenUsage,
}

impl TokenTracker {
    pub fn accumulate(&self, pair_key: Option<&str>, input: u64, output: u64) {
        self.global.accumulate(input, output);
        if let Some(key) = pair_key {
            if let Ok(mut map) = self.per_pair.lock() {
                let entry = map.entry(key.to_string()).or_default();
                entry.accumulate(input, output);
            }
        }
    }

    pub fn get_per_pair(&self, pair_key: &str) -> PairTokenUsage {
        self.per_pair
            .lock()
            .ok()
            .and_then(|m| m.get(pair_key).cloned())
            .unwrap_or_default()
    }

    pub fn reset(&self) {
        self.global.reset();
        if let Ok(mut map) = self.per_pair.lock() {
            map.clear();
        }
    }
}
