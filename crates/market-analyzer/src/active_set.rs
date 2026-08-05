use std::collections::HashSet;

use config_models::ActivationConfig;

#[derive(Debug, Clone)]
pub struct ActiveSet {
    pub disabled_indicators: HashSet<String>,
    pub disabled_signals: HashSet<(String, String)>,
    pub disabled_signal_kinds: HashSet<String>,
    pub config_version: u64,
    pub liquidity_enabled: bool,
    pub liquidation_feed: bool,
    pub cluster_estimation: bool,
    pub liquidity_signals_enabled: bool,
}

impl ActiveSet {
    pub fn all_enabled() -> Self {
        Self {
            disabled_indicators: HashSet::new(),
            disabled_signals: HashSet::new(),
            disabled_signal_kinds: HashSet::new(),
            config_version: 1,
            liquidity_enabled: true,
            liquidation_feed: true,
            cluster_estimation: true,
            liquidity_signals_enabled: true,
        }
    }

    pub fn from_config(
        global: &ActivationConfig,
        instance: Option<&ActivationConfig>,
        config_version: u64,
        // v6.10 (Phase 5 / E2): the master `[liquidity] enabled` toggle
        // is read from the global LiquidityConfig (not the per-instance
        // ActivationConfig) so operators can disable the entire liquidity
        // chain via `[liquidity] enabled = false` (CA-15).
        liquidity_config_enabled: bool,
    ) -> Self {
        let mut disabled_indicators: HashSet<String> =
            global.disabled_indicators.iter().cloned().collect();
        let mut disabled_signals: HashSet<(String, String)> = global
            .disabled_signals
            .iter()
            .map(|s| {
                let parts: Vec<&str> = s.splitn(2, ':').collect();
                (
                    parts[0].to_string(),
                    parts.get(1).map(|p| p.to_string()).unwrap_or_default(),
                )
            })
            .collect();
        let mut disabled_signal_kinds: HashSet<String> =
            global.disabled_signal_kinds.iter().cloned().collect();

        if let Some(inst) = instance {
            for ind in &inst.disabled_indicators {
                disabled_indicators.insert(ind.clone());
            }
            for sig in &inst.disabled_signals {
                let parts: Vec<&str> = sig.splitn(2, ':').collect();
                disabled_signals.insert((
                    parts[0].to_string(),
                    parts.get(1).map(|p| p.to_string()).unwrap_or_default(),
                ));
            }
            for kind in &inst.disabled_signal_kinds {
                disabled_signal_kinds.insert(kind.clone());
            }
        }

        Self {
            disabled_indicators,
            disabled_signals,
            disabled_signal_kinds,
            config_version,
            // v6.10 (Phase 5 / E2): wire the `[liquidity] enabled` master
            // toggle. When `false`, L1.5/L2.5/Phase 3 are all disabled
            // and to_metrics_config reports it accurately.
            liquidity_enabled: liquidity_config_enabled,
            // v6.10 (Phase 5 / E3): per-instance liquidity sub-toggles use
            // `Option<bool>`. `None` means inherit the global; `Some(v)`
            // means force the instance value. This lets operators opt
            // out of a sub-feature on a specific instance (e.g. disable
            // cluster estimation on macro while keeping liquidation feed).
            liquidation_feed: instance
                .and_then(|i| i.liquidation_feed)
                .or(global.liquidation_feed)
                .unwrap_or(true),
            cluster_estimation: instance
                .and_then(|i| i.cluster_estimation)
                .or(global.cluster_estimation)
                .unwrap_or(true),
            liquidity_signals_enabled: instance
                .and_then(|i| i.liquidity_signals_enabled)
                .or(global.liquidity_signals_enabled)
                .unwrap_or(true),
        }
    }

    pub fn is_indicator_enabled(&self, key: &str) -> bool {
        !self.disabled_indicators.contains(key)
    }

    pub fn is_signal_kind_enabled(&self, kind: &str) -> bool {
        !self.disabled_signal_kinds.contains(kind)
    }

    pub fn is_signal_pair_enabled(&self, indicator: &str, signal_kind: &str) -> bool {
        !self
            .disabled_signals
            .contains(&(indicator.to_string(), signal_kind.to_string()))
    }

    pub fn has_any_disabled(&self) -> bool {
        !self.disabled_indicators.is_empty()
            || !self.disabled_signals.is_empty()
            || !self.disabled_signal_kinds.is_empty()
    }

    pub fn to_metrics_config(&self) -> Option<core_domain::models::MetricsConfig> {
        if !self.has_any_disabled() {
            return None;
        }
        Some(core_domain::models::MetricsConfig {
            disabled_indicators: self.disabled_indicators.iter().cloned().collect(),
            disabled_signals: self
                .disabled_signals
                .iter()
                .map(|(ind, kind)| (ind.clone(), kind.clone()))
                .collect(),
            disabled_signal_kinds: self.disabled_signal_kinds.iter().cloned().collect(),
            liquidity: core_domain::models::LiquidityActivation {
                enabled: self.liquidity_enabled,
                liquidation_feed: self.liquidation_feed,
                cluster_estimation: self.cluster_estimation,
                signals: self.liquidity_signals_enabled,
            },
            config_version: self.config_version,
        })
    }
}

impl Default for ActiveSet {
    fn default() -> Self {
        Self::all_enabled()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_enabled_has_no_disabled_sets() {
        let s = ActiveSet::all_enabled();
        assert!(s.disabled_indicators.is_empty());
        assert!(s.disabled_signals.is_empty());
        assert!(s.disabled_signal_kinds.is_empty());
        assert!(s.liquidity_enabled);
        assert!(s.liquidation_feed);
        assert!(s.cluster_estimation);
        assert!(s.liquidity_signals_enabled);
    }

    #[test]
    fn default_matches_all_enabled() {
        let s = ActiveSet::default();
        assert_eq!(s.liquidity_enabled, ActiveSet::all_enabled().liquidity_enabled);
    }

    #[test]
    fn has_any_disabled_returns_true_when_indicator_listed() {
        let mut s = ActiveSet::all_enabled();
        s.disabled_indicators.insert("rsi".to_string());
        assert!(s.has_any_disabled());
    }

    #[test]
    fn liquidity_master_off_reaches_metrics_config() {
        use config_models::ActivationConfig;
        let global = ActivationConfig::default();
        // Master off: pass `false` for `liquidity_config_enabled`.
        let s = ActiveSet::from_config(&global, None, 1, false);
        assert!(!s.liquidity_enabled, "master off must disable liquidity");
        let cfg = s.to_metrics_config();
        if let Some(cfg) = cfg {
            assert!(!cfg.liquidity.enabled);
        }
    }
}
