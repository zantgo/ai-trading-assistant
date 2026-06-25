use std::collections::HashMap;

/// Support & Resistance Role-Reversal Engine (Section 2.3.2).
///
/// Tracks marked horizontal S/R levels and automatically flips their roles
/// when a 5-minute candle closes decisively beyond the level.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LevelRole {
    Support,
    Resistance,
}

#[derive(Debug, Clone)]
pub struct TrackedLevel {
    pub price: f64,
    pub role: LevelRole,
    pub original_role: LevelRole,
    pub last_flip_timestamp: Option<u64>,
    pub flip_count: u32,
}

#[derive(Debug, Clone)]
pub struct SrRoleTracker {
    levels: Vec<TrackedLevel>,
    flip_tolerance_pct: f64,
}

#[derive(Debug, Clone)]
pub struct FlipEvent {
    pub level_price: f64,
    pub from_role: LevelRole,
    pub to_role: LevelRole,
    pub candle_close: f64,
    pub candle_timestamp: u64,
}

impl SrRoleTracker {
    pub fn new(flip_tolerance_pct: f64) -> Self {
        Self {
            levels: Vec::new(),
            flip_tolerance_pct,
        }
    }

    /// Register or update tracked levels. Existing levels are merged.
    pub fn register_levels(&mut self, support_prices: &[f64], resistance_prices: &[f64]) {
        let mut new_levels: HashMap<i64, TrackedLevel> = HashMap::new();

        for &price in support_prices {
            let key = (price * 100.0) as i64;
            new_levels.entry(key).or_insert(TrackedLevel {
                price,
                role: LevelRole::Support,
                original_role: LevelRole::Support,
                last_flip_timestamp: None,
                flip_count: 0,
            });
        }

        for &price in resistance_prices {
            let key = (price * 100.0) as i64;
            new_levels.entry(key).or_insert(TrackedLevel {
                price,
                role: LevelRole::Resistance,
                original_role: LevelRole::Resistance,
                last_flip_timestamp: None,
                flip_count: 0,
            });
        }

        // Merge with existing: keep tracked flip state
        for existing in &self.levels {
            let key = (existing.price * 100.0) as i64;
            if !new_levels.contains_key(&key) {
                new_levels.insert(key, existing.clone());
            } else if let Some(entry) = new_levels.get_mut(&key) {
                entry.last_flip_timestamp = existing.last_flip_timestamp;
                entry.flip_count = existing.flip_count;
                entry.role = existing.role;
                entry.original_role = existing.original_role;
            }
        }

        self.levels = new_levels.into_values().collect();
        self.levels.sort_by(|a, b| {
            a.price
                .partial_cmp(&b.price)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    /// Process a 5-minute candle close. Returns any flip events.
    pub fn process_candle_close(&mut self, close_price: f64, timestamp: u64) -> Vec<FlipEvent> {
        let mut flips = Vec::new();
        let tolerance = close_price * self.flip_tolerance_pct;

        for level in &mut self.levels {
            match level.role {
                LevelRole::Resistance => {
                    // Resistance-to-Support flip: close decisively above resistance
                    if close_price > level.price && (close_price - level.price) > tolerance {
                        let event = FlipEvent {
                            level_price: level.price,
                            from_role: LevelRole::Resistance,
                            to_role: LevelRole::Support,
                            candle_close: close_price,
                            candle_timestamp: timestamp,
                        };
                        level.role = LevelRole::Support;
                        level.last_flip_timestamp = Some(timestamp);
                        level.flip_count += 1;
                        flips.push(event);
                    }
                }
                LevelRole::Support => {
                    // Support-to-Resistance flip: close decisively below support
                    if close_price < level.price && (level.price - close_price) > tolerance {
                        let event = FlipEvent {
                            level_price: level.price,
                            from_role: LevelRole::Support,
                            to_role: LevelRole::Resistance,
                            candle_close: close_price,
                            candle_timestamp: timestamp,
                        };
                        level.role = LevelRole::Resistance;
                        level.last_flip_timestamp = Some(timestamp);
                        level.flip_count += 1;
                        flips.push(event);
                    }
                }
            }
        }

        flips
    }

    /// Get current support levels (including flipped ones).
    pub fn get_supports(&self) -> Vec<f64> {
        self.levels
            .iter()
            .filter(|l| l.role == LevelRole::Support)
            .map(|l| l.price)
            .collect()
    }

    /// Get current resistance levels (including flipped ones).
    pub fn get_resistances(&self) -> Vec<f64> {
        self.levels
            .iter()
            .filter(|l| l.role == LevelRole::Resistance)
            .map(|l| l.price)
            .collect()
    }

    /// Get all tracked levels.
    pub fn get_all_levels(&self) -> &[TrackedLevel] {
        &self.levels
    }

    /// Check if a specific price is currently acting as support.
    pub fn is_support(&self, price: f64) -> bool {
        self.levels
            .iter()
            .any(|l| (l.price - price).abs() < 0.01 && l.role == LevelRole::Support)
    }

    /// Check if a specific price is currently acting as resistance.
    pub fn is_resistance(&self, price: f64) -> bool {
        self.levels
            .iter()
            .any(|l| (l.price - price).abs() < 0.01 && l.role == LevelRole::Resistance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resistance_to_support_flip() {
        let mut tracker = SrRoleTracker::new(0.005);
        tracker.register_levels(&[100.0], &[110.0, 120.0]);

        let flips = tracker.process_candle_close(111.0, 1000);
        assert_eq!(flips.len(), 1);
        assert_eq!(flips[0].from_role, LevelRole::Resistance);
        assert_eq!(flips[0].to_role, LevelRole::Support);
        assert_eq!(flips[0].level_price, 110.0);

        assert!(tracker.is_support(110.0));
        assert!(!tracker.is_resistance(110.0));
    }

    #[test]
    fn test_support_to_resistance_flip() {
        let mut tracker = SrRoleTracker::new(0.005);
        tracker.register_levels(&[100.0], &[120.0]);

        let flips = tracker.process_candle_close(99.0, 1000);
        assert_eq!(flips.len(), 1);
        assert_eq!(flips[0].from_role, LevelRole::Support);
        assert_eq!(flips[0].to_role, LevelRole::Resistance);
    }

    #[test]
    fn test_no_flip_within_tolerance() {
        let mut tracker = SrRoleTracker::new(0.005);
        tracker.register_levels(&[100.0], &[110.0]);

        let flips = tracker.process_candle_close(110.3, 1000);
        assert!(flips.is_empty());
    }

    #[test]
    fn test_register_levels_merges_flip_state() {
        let mut tracker = SrRoleTracker::new(0.005);
        tracker.register_levels(&[100.0, 105.0], &[110.0]);

        let _ = tracker.process_candle_close(111.0, 1000);
        assert!(tracker.is_support(110.0));

        tracker.register_levels(&[100.0, 105.0], &[110.0]);
        assert!(tracker.is_support(110.0));
    }
}
