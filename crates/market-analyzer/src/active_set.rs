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
    ) -> Self {
        let mut disabled_indicators: HashSet<String> =
            global.disabled_indicators.iter().cloned().collect();
        let mut disabled_signals: HashSet<(String, String)> = global
            .disabled_signals
            .iter()
            .map(|s| {
                let parts: Vec<&str> = s.splitn(2, ':').collect();
                (parts[0].to_string(), parts.get(1).map(|p| p.to_string()).unwrap_or_default())
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
                disabled_signals.insert((parts[0].to_string(), parts.get(1).map(|p| p.to_string()).unwrap_or_default()));
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
            liquidity_enabled: true,
            liquidation_feed: instance.map(|i| i.liquidation_feed).unwrap_or(global.liquidation_feed),
            cluster_estimation: instance.map(|i| i.cluster_estimation).unwrap_or(global.cluster_estimation),
            liquidity_signals_enabled: instance.map(|i| i.liquidity_signals_enabled).unwrap_or(global.liquidity_signals_enabled),
        }
    }

    pub fn is_indicator_enabled(&self, key: &str) -> bool {
        !self.disabled_indicators.contains(key)
    }

    pub fn is_signal_kind_enabled(&self, kind: &str) -> bool {
        !self.disabled_signal_kinds.contains(kind)
    }

    pub fn is_signal_pair_enabled(&self, indicator: &str, signal_kind: &str) -> bool {
        !self.disabled_signals.contains(&(indicator.to_string(), signal_kind.to_string()))
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
