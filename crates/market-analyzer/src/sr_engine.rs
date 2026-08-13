use std::collections::HashMap;

/// Support & Resistance Role-Reversal Engine (Section 2.3.2).
///
/// Tracks marked horizontal S/R levels and automatically flips their roles
/// when a 5-minute candle closes decisively beyond the level.

/// AUDIT-AIU-006: levels are keyed on **4 significant digits** (scale-aware)
/// instead of `(price × 100.0) as i64`, which collapsed every level below
/// $0.01 to key 0 (one level for sub-cent assets like SHIB) and truncated-
/// merged any two levels within the same cent. The relative tolerance for
/// proximity checks (`is_support` / `is_resistance`) is 0.05% of the level,
/// also scale-aware.
const MIN_LEVEL_GAP_PCT: f64 = 0.0005;

/// Scale-aware dedup key: rounds `price` to 4 significant digits.
fn level_key(price: f64) -> i64 {
    if !price.is_finite() || price <= 0.0 {
        return 0;
    }
    let mag = price.abs().log10().floor() as i32;
    let scale = 10f64.powi(4 - mag);
    (price * scale).round() as i64
}

/// Merge levels closer than 0.05% relative: keeps the first registered level
/// (supports before resistances) and drops the duplicate.
fn merge_proximate(levels: &mut Vec<TrackedLevel>) {
    if levels.len() < 2 {
        return;
    }
    levels.sort_by(|a, b| {
        a.price
            .partial_cmp(&b.price)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut i = 0;
    while i + 1 < levels.len() {
        let a = levels[i].price;
        let b = levels[i + 1].price;
        if b - a <= a * MIN_LEVEL_GAP_PCT {
            // Keep the stronger entry: prefer the one with the higher flip
            // count (longer-confirmed role); on ties keep the first.
            let keep = if levels[i + 1].flip_count > levels[i].flip_count {
                i + 1
            } else {
                i
            };
            let drop = if keep == i { i + 1 } else { i };
            if drop == i {
                i += 1; // removed the earlier entry; re-check position
            } else {
                levels.remove(drop);
            }
        } else {
            i += 1;
        }
    }
}

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
            let key = level_key(price);
            new_levels.entry(key).or_insert(TrackedLevel {
                price,
                role: LevelRole::Support,
                original_role: LevelRole::Support,
                last_flip_timestamp: None,
                flip_count: 0,
            });
        }

        for &price in resistance_prices {
            let key = level_key(price);
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
            let key = level_key(existing.price);
            if let std::collections::hash_map::Entry::Vacant(e) = new_levels.entry(key) {
                e.insert(existing.clone());
            } else if let Some(entry) = new_levels.get_mut(&key) {
                entry.last_flip_timestamp = existing.last_flip_timestamp;
                entry.flip_count = existing.flip_count;
                entry.role = existing.role;
                entry.original_role = existing.original_role;
            }
        }

        let mut merged: Vec<TrackedLevel> = new_levels.into_values().collect();
        // AUDIT-AIU-006: collapse levels within the relative merge gap.
        merge_proximate(&mut merged);
        self.levels = merged;
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
    /// AUDIT-AIU-006: relative 0.05% tolerance (was a fixed $0.01).
    pub fn is_support(&self, price: f64) -> bool {
        self.levels
            .iter()
            .any(|l| (l.price - price).abs() <= l.price * MIN_LEVEL_GAP_PCT && l.role == LevelRole::Support)
    }

    /// Check if a specific price is currently acting as resistance.
    /// AUDIT-AIU-006: relative 0.05% tolerance (was a fixed $0.01).
    pub fn is_resistance(&self, price: f64) -> bool {
        self.levels
            .iter()
            .any(|l| (l.price - price).abs() <= l.price * MIN_LEVEL_GAP_PCT && l.role == LevelRole::Resistance)
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

    #[test]
    fn test_sub_cent_levels_do_not_collapse() {
        // AUDIT-AIU-006: prices below $0.01 previously all keyed to 0.
        let mut tracker = SrRoleTracker::new(0.005);
        tracker.register_levels(&[0.00001, 0.00002, 0.00003], &[]);
        let levels = tracker.get_supports();
        assert_eq!(levels.len(), 3, "sub-cent levels must stay distinct");
        assert!(levels.contains(&0.00001));
        assert!(levels.contains(&0.00002));
        assert!(levels.contains(&0.00003));
    }

    #[test]
    fn test_same_cent_levels_merge_within_gap() {
        // Two levels 0.002% apart (< 0.05% gap) collapse to one.
        let mut tracker = SrRoleTracker::new(0.005);
        tracker.register_levels(&[100.004, 100.006], &[]);
        assert_eq!(tracker.get_supports().len(), 1);
    }

    #[test]
    fn test_relative_proximity_for_sub_cent() {
        let mut tracker = SrRoleTracker::new(0.005);
        tracker.register_levels(&[0.00001], &[]);
        // 0.000010001 is within 0.05% of 0.00001.
        assert!(tracker.is_support(0.000010001));
        // 0.00002 is far outside the 0.05% gap.
        assert!(!tracker.is_support(0.00002));
    }
}
