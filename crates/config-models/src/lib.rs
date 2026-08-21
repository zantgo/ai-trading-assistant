pub mod models;
pub use models::*;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error type for all config-loader failures. Replaces the previous
/// pattern of `.expect("...")` calls, which mixed parser errors with IO
/// errors and made recovery impossible at boot time.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("could not read `{path}`: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("TOML syntax error in `{path}`: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("TOML serialization error: {0}")]
    Serialize(#[from] toml::ser::Error),

    #[error(
        "legacy config file `{path}` is no longer recognized.\n\
         The platform now reads `config.toml` only. Migrate your\n\
         settings to the new schema documented in\n\
         `docs/conceptual-foundations/01-07-data-model-hierarchy.md`."
    )]
    LegacyFile { path: PathBuf },

    #[error(
        "workspace table is missing from `config.toml`. The platform\n\
         requires a `[workspace]` section. See\n\
         `docs/conceptual-foundations/01-07-data-model-hierarchy.md`."
    )]
    WorkspaceMissing,

    #[error(
        "instance `{symbol}` declares {count} custom timeframes ({keys}),\n\
         but custom pipeline slots are not yet instantiated by the runtime\n\
         (see `docs/ROADMAP.md` §3 Phase A — custom `instances[*].custom_pipelines`).\n\
         Remove the `custom_pipelines` table or restrict the instance to the\n\
         default 4-slot ladder (micro/fast/slow/macro) to boot."
    )]
    CustomTimeframesUnsupported {
        symbol: String,
        count: usize,
        keys: String,
    },

    #[error(
        "invalid numeric config (audit M8): {detail}.\n\
         Zero-valued periods/durations panic in the hot path (Decimal/u64\n\
         division, median-window indexing) — every period must be >= 1."
    )]
    InvalidNumeric { detail: String },
}

/// Alias for `Result<T, ConfigError>`.
pub type Result<T> = std::result::Result<T, ConfigError>;

/// On-disk shape of `config.toml`. Serde-deserialized directly.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OnDiskConfig {
    #[serde(default)]
    hyperliquid: HyperliquidConfig,
    #[serde(default)]
    bitget: BitgetConfig,
    #[serde(default)]
    clock_monitor: Option<ClockMonitorTomlConfig>,
    #[serde(default)]
    quality: Option<QualityConfig>,
    #[serde(default)]
    reconnect: ReconnectConfig,
    #[serde(default)]
    candle_buffer: CandleBufferConfig,
    /// Optional snapshot-export scheduler. When `None` the
    /// `SnapshotExportConfig::default()` (disabled) is used.
    #[serde(default)]
    snapshot_export: Option<SnapshotExportConfig>,
    workspace: WorkspaceConfig,
}

impl OnDiskConfig {
    /// Decompose into `(PlatformConfig, WorkspaceConfig)`.
    fn split(self) -> (PlatformConfig, WorkspaceConfig) {
        (
            PlatformConfig {
                hyperliquid: self.hyperliquid,
                bitget: self.bitget,
                clock_monitor: self.clock_monitor,
                quality: self.quality,
                reconnect: self.reconnect,
                candle_buffer: self.candle_buffer,
                snapshot_export: self.snapshot_export.unwrap_or_default(),
            },
            self.workspace,
        )
    }
}

/// Platform-level configuration. Read once at startup by `execution-daemon`.
/// Contains the things that are NOT per-workspace / per-instance: the
/// exchange endpoints the binary connects to and the NTP clock monitor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PlatformConfig {
    #[serde(default)]
    pub hyperliquid: HyperliquidConfig,
    #[serde(default)]
    pub bitget: BitgetConfig,
    /// Optional clock-drift monitor. When `Some` and `is_active()`, main.rs
    /// spawns the NTP-based monitor alongside the other background tasks.
    #[serde(default)]
    pub clock_monitor: Option<ClockMonitorTomlConfig>,

    /// Optional data-quality configuration (median filter, outlier tolerance).
    /// When `None`, the median filter is disabled and all ticks are accepted.
    #[serde(default)]
    pub quality: Option<QualityConfig>,
    #[serde(default)]
    pub reconnect: ReconnectConfig,
    /// Single source of truth for candle buffer behavior. Replaces the
    /// previous per-instance `analysis_limit` field. See
    /// `docs/operations-and-compliance/08-08-candle-buffer-spec.md` (CB-01).
    #[serde(default)]
    pub candle_buffer: CandleBufferConfig,
    /// Periodic per-tab JSON dump configuration. See
    /// `SnapshotExportConfig` and `docs/operations-and-compliance/08-09-snapshot-export.md`.
    /// Default `SnapshotExportConfig::default()` (disabled) is used when
    /// the `[snapshot_export]` section is absent from `config.toml`.
    #[serde(default)]
    pub snapshot_export: SnapshotExportConfig,
}

/// Status of a single trading-pair instance. Persisted in the workspace file
/// so the dashboard can render the row correctly after a restart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum InstanceStatus {
    #[default]
    Running,
    Paused,
    Stopped,
}

/// One workspace = one portfolio + analytics + strategies + market-monitor
/// settings + an array of trading-pair instances.
///
/// "All engines running my program": the workspace is the unit of ownership
/// for the user's portfolio. Exactly one workspace per binary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// Workspace identifier (slug, filesystem-safe). Currently always
    /// `"main"` — the binary supports one workspace per deployment.
    pub id: String,
    /// Display name shown in the dashboard header.
    pub name: String,
    /// Default settlement currency for new instances.
    pub default_currency: String,
    /// Default exchange for new instances.
    pub default_exchange: String,

    // ─── Market-monitor defaults (per-instance inheritance) ────────
    #[serde(default)]
    pub candles: CandlesConfig,
    #[serde(default)]
    pub indicators: IndicatorsConfig,
    #[serde(default)]
    pub fast_timeframe: FastTimeframeConfig,
    #[serde(default)]
    pub slow_timeframe: SlowTimeframeConfig,
    #[serde(default)]
    pub macro_timeframe: SlowTimeframeConfig,
    #[serde(default)]
    pub fibonacci: FibonacciConfig,
    #[serde(default)]
    pub pivots: PivotsConfig,

    // ─── Portfolio / strategy / analytics ──────────────────────────
    #[serde(default)]
    pub safety: SafetyConfig,
    #[serde(default)]
    pub fees: FeesConfig,
    #[serde(default)]
    pub intervals: IntervalsConfig,
    #[serde(default)]
    pub liquidity: LiquidityConfig,
    #[serde(default)]
    pub heatmap: HeatmapConfig,
    #[serde(default)]
    pub api_failover: ApiFailoverConfig,
    #[serde(default)]
    pub activation: ActivationConfig,
    /// Opportunity-matrix knobs — currently just the ATR-fallback toggle
    /// for confluent levels (Phase C of the v6.10 fix). When `enabled`,
    /// the synthesis emits at least one entry / target level derived from
    /// `close ± k·ATR` if every structural source (Fibonacci / Volume
    /// Profile / Pivot Points / Liquidation Clusters) is empty, so the
    /// Opportunities panel never shows "No confluent levels" for a
    /// healthy market. When `disabled` (strict behaviour), the empty
    /// state is the honest signal of "no structural levels near price".
    #[serde(default)]
    pub opportunity_matrix: OpportunityMatrixConfig,
    /// Schema version counter — incremented on every successful POST /api/config.
    #[serde(default)]
    pub config_version: u64,
    #[serde(default)]
    pub scoring: ScoringConfig,
    #[serde(default)]
    pub leverage: LeverageConfig,
    #[serde(default)]
    pub defaults: DefaultsConfig,

    /// Zero or more trading-pair instances.
    #[serde(default)]
    pub instances: Vec<InstanceEntry>,

    /// v7 Trade Automation Engine — minimal setup-executor configuration.
    #[serde(default)]
    pub minimal_tae: MinimalTaeConfig,

    /// v7.3 PAE significance-treatment configuration (α, Monte Carlo runs,
    /// min-trades for the edge verdict).
    #[serde(default)]
    pub analytics: AnalyticsConfig,

    /// v7.3 portfolio risk limits — concentration / exposure / correlation
    /// caps the PME Exposure layer enforces and the dashboard renders.
    #[serde(default)]
    pub risk_limits: RiskLimitsConfig,

    /// Execution-layer configuration (slippage ceiling, etc.).
    #[serde(default)]
    pub execution: ExecutionConfig,

    /// Backtesting Engine (BTE) — candle archive depth, warmup bars,
    /// per-exchange paging limits for the deep-history backtest.
    #[serde(default)]
    pub backtest: BacktestConfig,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            id: "main".to_string(),
            name: "Default Workspace".to_string(),
            default_currency: "USDC".to_string(),
            default_exchange: "Hyperliquid".to_string(),
            candles: CandlesConfig::default(),
            indicators: IndicatorsConfig::default(),
            fast_timeframe: FastTimeframeConfig::default(),
            slow_timeframe: SlowTimeframeConfig::default(),
            macro_timeframe: SlowTimeframeConfig::default(),
            fibonacci: FibonacciConfig::default(),
            pivots: PivotsConfig::default(),
            safety: SafetyConfig::default(),
            fees: FeesConfig::default(),
            intervals: IntervalsConfig::default(),
            liquidity: LiquidityConfig::default(),
            heatmap: HeatmapConfig::default(),
            api_failover: ApiFailoverConfig::default(),
            activation: ActivationConfig::default(),
            opportunity_matrix: OpportunityMatrixConfig::default(),
            config_version: 1,
            scoring: ScoringConfig::default(),
            leverage: LeverageConfig::default(),
            defaults: DefaultsConfig::default(),
            instances: Vec::new(),
            minimal_tae: MinimalTaeConfig::default(),
            analytics: AnalyticsConfig::default(),
            risk_limits: RiskLimitsConfig::default(),
            execution: ExecutionConfig::default(),
            backtest: BacktestConfig::default(),
        }
    }
}

impl WorkspaceConfig {
    /// Convenience: the set of symbols that the workspace declares it should
    /// be running (extracted from the `instances[].symbol` list). Useful for
    /// the dashboard's "what pairs are configured" panel.
    pub fn declared_symbols(&self) -> Vec<String> {
        self.instances.iter().map(|i| i.symbol.clone()).collect()
    }

    /// v7.2 parity: the canonical default timeframe ladder — the SAME
    /// values the registry falls back to when an instance is created
    /// without a config entry (`registry::add_instance`): micro 60s,
    /// fast 180s, slow/macro from the workspace defaults. The Launch
    /// Setup wizard and the CLI launch prompt derive their per-instance
    /// defaults from this ladder, so every surface agrees on the default
    /// pipeline durations.
    pub fn tf_ladder_defaults(&self) -> (u64, u64, u64, u64) {
        (
            60,
            180,
            self.slow_timeframe.duration_seconds,
            self.macro_timeframe.duration_seconds,
        )
    }
}

/// One trading-pair instance inside a workspace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstanceEntry {
    /// Stable identifier used as the runtime key (e.g. `"btc"` or `"BTC-USDT"`).
    pub id: String,
    /// Exchange-native symbol (e.g. `"BTC-USDT"`, `"ETH-USDC"`).
    pub symbol: String,
    /// Quote currency for this instance. Usually matches the workspace's
    /// `default_currency`.
    #[serde(default)]
    pub quote: String,
    /// Initial capital allocation for this instance (USD).
    #[serde(default = "default_initial_capital")]
    pub initial_capital_usd: f64,
    /// Runtime status: running / paused / stopped. Defaults to running; the
    /// dashboard flips the bit when the user pauses or stops the instance.
    #[serde(default)]
    pub status: InstanceStatus,
    pub micro_term: TimeframeConfig,
    pub fast_term: TimeframeConfig,
    #[serde(default)]
    pub slow_term: Option<TimeframeConfig>,
    #[serde(default)]
    pub macro_term: Option<TimeframeConfig>,
    #[serde(default)]
    pub automation: AutomationConfig,
    #[serde(default)]
    pub operational_mode: OperationalMode,
    /// v7 execution mode (Observe / Paper / Live). Default Paper.
    #[serde(default)]
    pub mode: ExecutionMode,
    #[serde(default)]
    pub weight_overrides: Option<std::collections::HashMap<String, i32>>,
    #[serde(default)]
    pub position_scaling: Option<PositionScalingConfig>,
    /// Per-instance activation overrides (union with global [activation]).
    #[serde(default)]
    pub activation: Option<ActivationConfig>,
    /// Operator-defined custom slot pipelines (`TimeframeSlot::Custom { id }`).
    /// Empty for the default 4-slot ladder. The registry maps `id → name`
    /// and the `TimeframeSlot::Custom { id }` enum variant carries the index
    /// on the wire. Default is empty for backward compatibility.
    #[serde(default)]
    pub custom_pipelines: std::collections::HashMap<u16, TimeframeConfig>,
}

fn default_initial_capital() -> f64 {
    1_000.0
}

/// Execution mode for the unified execution engine. The mode only affects
/// the final broker dispatch: `Observe` never submits orders (advisory /
/// market-monitoring only), `Paper` simulates fills internally, `Live`
/// routes to an exchange. All accounting (fees, slippage, funding, PnL) is
/// identical in `Paper` and `Live`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum ExecutionMode {
    /// Market/signal monitoring only — no orders are ever dispatched.
    Observe,
    #[default]
    Paper,
    Live,
}

/// v7 TAE — the minimal setup-executor configuration. Replaces the erased
/// policy engine: the executor consumes the MME's top setup directly and
/// manages the trade to completion. See docs/engines/trade-automation-engine/.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MinimalTaeConfig {
    /// Master switch for the setup executor.
    #[serde(default)]
    pub enabled: bool,
    /// Percent of instance equity risked per trade (1.0 = 1%).
    #[serde(default = "default_risk_per_trade_pct")]
    pub risk_per_trade_pct: f64,
    /// Fee-adjusted minimum reward-to-risk ratio for accepting a setup.
    #[serde(default = "default_min_net_rr")]
    pub min_net_rr: f64,
    /// Optional notional cap (USD). None = no cap.
    #[serde(default)]
    pub max_position_size_usd: Option<f64>,
    /// Global concurrent-position cap across all symbols.
    #[serde(default = "default_max_open_positions")]
    pub max_open_positions: u32,
    /// Entry placement mode. v7 supports only "zone_midpoint".
    #[serde(default = "default_entry_mode")]
    pub entry_mode: String,
    /// Invalidation semantics for open positions. v7 default: strict
    /// opposite-direction flip only ("direction_flip").
    #[serde(default = "default_invalidate_on")]
    pub invalidate_on: String,
}

fn default_risk_per_trade_pct() -> f64 {
    1.0
}
fn default_min_net_rr() -> f64 {
    1.0
}
fn default_max_open_positions() -> u32 {
    1
}
fn default_entry_mode() -> String {
    "zone_midpoint".to_string()
}
fn default_invalidate_on() -> String {
    "direction_flip".to_string()
}

impl Default for MinimalTaeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            risk_per_trade_pct: default_risk_per_trade_pct(),
            min_net_rr: default_min_net_rr(),
            max_position_size_usd: None,
            max_open_positions: default_max_open_positions(),
            entry_mode: default_entry_mode(),
            invalidate_on: default_invalidate_on(),
        }
    }
}

// ===========================================================================
// Loaders
// ===========================================================================

/// Canonical config path. Allows tests and the `manage.sh` wrapper to
/// override the location via `MARKET_MONITOR_CONFIG` for staging.
fn config_path() -> PathBuf {
    if let Ok(p) = std::env::var("MARKET_MONITOR_CONFIG") {
        return PathBuf::from(p);
    }
    PathBuf::from("config.toml")
}

/// Check that none of the legacy config files are present. If any are,
/// return `LegacyFile` so the caller can panic with a migration pointer.
fn assert_no_legacy_files() -> Result<()> {
    for legacy in ["instances.json", "workspaces.json", "pairs.json"] {
        if Path::new(legacy).exists() {
            return Err(ConfigError::LegacyFile {
                path: PathBuf::from(legacy),
            });
        }
    }
    Ok(())
}

/// Load the platform config from `config.toml`.
pub fn load_platform() -> Result<PlatformConfig> {
    assert_no_legacy_files()?;
    let path = config_path();
    let raw = std::fs::read_to_string(&path).map_err(|e| ConfigError::Io {
        path: path.clone(),
        source: e,
    })?;
    let on_disk: OnDiskConfig = toml::from_str(&raw).map_err(|e| ConfigError::Parse {
        path: path.clone(),
        source: e,
    })?;
    let (platform, _workspace) = on_disk.split();
    validate_platform(&platform)?;
    Ok(platform)
}

/// Load the workspace config from `config.toml` (the `[workspace]` table).
pub fn load_workspace() -> Result<WorkspaceConfig> {
    assert_no_legacy_files()?;
    let path = config_path();
    let raw = std::fs::read_to_string(&path).map_err(|e| ConfigError::Io {
        path: path.clone(),
        source: e,
    })?;
    let on_disk: OnDiskConfig = toml::from_str(&raw).map_err(|e| ConfigError::Parse {
        path: path.clone(),
        source: e,
    })?;
    validate_workspace(&on_disk.workspace)?;
    Ok(on_disk.workspace)
}

/// Load both at once (the common case).
pub fn load() -> Result<(PlatformConfig, WorkspaceConfig)> {
    assert_no_legacy_files()?;
    let path = config_path();
    let raw = std::fs::read_to_string(&path).map_err(|e| ConfigError::Io {
        path: path.clone(),
        source: e,
    })?;
    let on_disk: OnDiskConfig = toml::from_str(&raw).map_err(|e| ConfigError::Parse {
        path: path.clone(),
        source: e,
    })?;
    validate_workspace(&on_disk.workspace)?;
    Ok(on_disk.split())
}

/// Fail-fast boot validation (audit fix M6): surfaces config surfaces the
/// runtime cannot honor instead of silently ignoring them.
///
/// Currently rejected: `InstanceEntry.custom_pipelines` — the registry has
/// full PRI-07 code paths for custom slots (cluster handles, history,
/// per-TF refresh) but no production call-site instantiates them, so a
/// configured custom TF would be silently dropped. Explicit rejection is
/// the honest behaviour until the wiring lands.
fn validate_workspace(ws: &WorkspaceConfig) -> Result<()> {
    // v7.3 (M8-style numeric guards): the significance treatment and the
    // risk limits are real numerics that flow into division/ranking logic —
    // reject nonsense at boot instead of silently mis-verdicting trades.
    if !(ws.analytics.alpha.is_finite() && ws.analytics.alpha > 0.0 && ws.analytics.alpha <= 1.0) {
        return Err(ConfigError::InvalidNumeric {
            detail: format!("[workspace.analytics].alpha = {} (must be in (0, 1])", ws.analytics.alpha),
        });
    }
    if ws.analytics.monte_carlo_runs < 1000 {
        return Err(ConfigError::InvalidNumeric {
            detail: format!(
                "[workspace.analytics].monte_carlo_runs = {} (must be >= 1000)",
                ws.analytics.monte_carlo_runs
            ),
        });
    }
    if ws.analytics.min_trades_for_verdict < 10 {
        return Err(ConfigError::InvalidNumeric {
            detail: format!(
                "[workspace.analytics].min_trades_for_verdict = {} (must be >= 10)",
                ws.analytics.min_trades_for_verdict
            ),
        });
    }
    // BTE (v8): archive depth 1..=365, warmup floor, and per-exchange
    // paging sanity. The depth is the "how far back can I look" contract —
    // reject out-of-range values instead of silently clamping.
    if !(1..=365).contains(&ws.backtest.archive_depth_days) {
        return Err(ConfigError::InvalidNumeric {
            detail: format!(
                "[workspace.backtest].archive_depth_days = {} (must be in 1..=365)",
                ws.backtest.archive_depth_days
            ),
        });
    }
    if ws.backtest.warmup_bars < 30 {
        return Err(ConfigError::InvalidNumeric {
            detail: format!(
                "[workspace.backtest].warmup_bars = {} (must be >= 30)",
                ws.backtest.warmup_bars
            ),
        });
    }
    if ws.backtest.max_equity_points < 10 {
        return Err(ConfigError::InvalidNumeric {
            detail: format!(
                "[workspace.backtest].max_equity_points = {} (must be >= 10)",
                ws.backtest.max_equity_points
            ),
        });
    }
    for (exchange, limits) in [
        ("hyperliquid", &ws.backtest.hyperliquid),
        ("bitget", &ws.backtest.bitget),
    ] {
        if limits.page_cap == 0 {
            return Err(ConfigError::InvalidNumeric {
                detail: format!(
                    "[workspace.backtest].{exchange}.page_cap = {} (must be > 0)",
                    limits.page_cap
                ),
            });
        }
        if limits.max_pages_per_run == 0 {
            return Err(ConfigError::InvalidNumeric {
                detail: format!(
                    "[workspace.backtest].{exchange}.max_pages_per_run = {} (must be >= 1)",
                    limits.max_pages_per_run
                ),
            });
        }
    }
    for (name, v) in [
        ("max_single_pair_exposure_pct", ws.risk_limits.max_single_pair_exposure_pct),
        ("max_portfolio_exposure_pct", ws.risk_limits.max_portfolio_exposure_pct),
    ] {
        if !v.is_finite() || !(0.0 < v) || !(v <= 100.0) {
            return Err(ConfigError::InvalidNumeric {
                detail: format!("[workspace.risk_limits].{name} = {v} (must be in (0, 100])"),
            });
        }
    }
    if !(0.0 < ws.risk_limits.max_correlation) || ws.risk_limits.max_correlation > 1.0 {
        return Err(ConfigError::InvalidNumeric {
            detail: format!(
                "[workspace.risk_limits].max_correlation = {} (must be in (0, 1])",
                ws.risk_limits.max_correlation
            ),
        });
    }
    for inst in &ws.instances {
        if !inst.custom_pipelines.is_empty() {
            let mut keys: Vec<String> = inst
                .custom_pipelines
                .keys()
                .map(|k| k.to_string())
                .collect();
            keys.sort();
            return Err(ConfigError::CustomTimeframesUnsupported {
                symbol: inst.symbol.clone(),
                count: inst.custom_pipelines.len(),
                keys: keys.join(", "),
            });
        }
        // M8 (production audit): zero-valued numeric knobs panic in the
        // hot path — `candles.duration_seconds = 0` divides by zero in
        // CandleGenerator, `rsi_period = 0` in Rsi::update, and
        // `median_window_size = 0` indexes an empty window in
        // MedianPriceFilter. Fail fast at boot instead.
        for (name, tf) in [
            ("micro", Some(&inst.micro_term)),
            ("fast", Some(&inst.fast_term)),
            ("slow", inst.slow_term.as_ref()),
            ("macro", inst.macro_term.as_ref()),
        ] {
            if let Some(tf) = tf {
                if tf.candles.duration_seconds == 0 {
                    return Err(ConfigError::InvalidNumeric {
                        detail: format!(
                            "instance {}: {}.candles.duration_seconds = 0",
                            inst.symbol, name
                        ),
                    });
                }
                let ind = &tf.indicators;
                if ind.rsi_period == 0 {
                    return Err(ConfigError::InvalidNumeric {
                        detail: format!("instance {}: {}.rsi_period = 0", inst.symbol, name),
                    });
                }
                if ind.macd_fast == 0 || ind.macd_slow == 0 || ind.macd_signal == 0 {
                    return Err(ConfigError::InvalidNumeric {
                        detail: format!("instance {}: {} MACD period(s) = 0", inst.symbol, name),
                    });
                }
            }
        }
    }
    Ok(())
}

/// M8: platform-level numeric guards (`load_platform` path) — the median
/// filter window must be ≥ 1.
fn validate_platform(platform: &PlatformConfig) -> Result<()> {
    if let Some(q) = &platform.quality {
        if q.median_window_size == 0 {
            return Err(ConfigError::InvalidNumeric {
                detail: "[quality].median_window_size = 0 (must be >= 1)".into(),
            });
        }
    }
    Ok(())
}

/// Serialize a `WorkspaceConfig` back to TOML and persist to `config.toml`.
///
/// The platform-level fields (exchanges, clock monitor) are not overwritten:
/// we read the current file, mutate the `[workspace]` table, and write the
/// file back. This preserves any platform-level edits the operator made
/// outside the workspace UI.
pub fn save_workspace(workspace: &WorkspaceConfig) -> Result<()> {
    assert_no_legacy_files()?;
    let path = config_path();

    // Re-read the file so we preserve the [platform] section unchanged.
    let raw = std::fs::read_to_string(&path).map_err(|e| ConfigError::Io {
        path: path.clone(),
        source: e,
    })?;
    let on_disk: OnDiskConfig = toml::from_str(&raw).map_err(|e| ConfigError::Parse {
        path: path.clone(),
        source: e,
    })?;
    let new_raw = OnDiskConfig {
        hyperliquid: on_disk.hyperliquid,
        bitget: on_disk.bitget,
        clock_monitor: on_disk.clock_monitor,
        quality: on_disk.quality,
        reconnect: on_disk.reconnect,
        candle_buffer: on_disk.candle_buffer,
        snapshot_export: on_disk.snapshot_export,
        workspace: workspace.clone(),
    };
    let serialized = toml::to_string_pretty(&new_raw)?;
    std::fs::write(&path, serialized).map_err(|e| ConfigError::Io {
        path: path.clone(),
        source: e,
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_config() {
        let toml = r#"
[workspace]
id = "main"
name = "Test"
default_currency = "USDC"
default_exchange = "Hyperliquid"

[[workspace.instances]]
id = "btc"
symbol = "BTC-USDT"
quote = "USDT"

[workspace.instances.micro_term]
candles = { duration_seconds = 60 }

[workspace.instances.fast_term]
candles = { duration_seconds = 180 }
"#;
        let cfg: OnDiskConfig = toml::from_str(toml).expect("parse");
        let (platform, workspace) = cfg.split();
        assert_eq!(workspace.id, "main");
        assert_eq!(workspace.instances.len(), 1);
        assert_eq!(workspace.instances[0].symbol, "BTC-USDT");
        assert_eq!(workspace.candles.duration_seconds, 60); // default
        assert!(platform.clock_monitor.is_none());
    }

    #[test]
    fn parse_partial_instance_indicator_override() {
        let toml = r#"
[workspace]
id = "main"
name = "Test"
default_currency = "USDC"
default_exchange = "Hyperliquid"

[[workspace.instances]]
id = "btc"
symbol = "BTC-USDT"
quote = "USDT"

[workspace.instances.micro_term]
candles = { duration_seconds = 60 }
indicators = { rsi_period = 21 }

[workspace.instances.fast_term]
candles = { duration_seconds = 180 }
indicators = { rsi_period = 14 }
"#;
        let cfg: OnDiskConfig = toml::from_str(toml).expect("partial indicators must parse");
        let (_platform, workspace) = cfg.split();
        let micro = &workspace.instances[0].micro_term.indicators;
        assert_eq!(micro.rsi_period, 21);
        assert_eq!(micro.ema_fast, 10);
        assert_eq!(micro.ema_long, 200);
        assert_eq!(micro.macd_slow, 26);
        assert_eq!(micro.squeeze_period, 20);
    }

    #[test]
    fn indicators_default_is_not_zero() {
        let cfg = IndicatorsConfig::default();
        assert_eq!(cfg.ema_fast, 10);
        assert_eq!(cfg.ema_medium, 50);
        assert_eq!(cfg.ema_slow, 100);
        assert_eq!(cfg.ema_long, 200);
        assert_eq!(cfg.rsi_period, 14);
        assert_eq!(cfg.macd_fast, 12);
        assert_eq!(cfg.macd_slow, 26);
        assert_eq!(cfg.macd_signal, 9);
        assert_eq!(cfg.adx_period, 14);
        assert_eq!(cfg.atr_period, 14);
        assert_eq!(cfg.squeeze_period, 20);
    }

    #[test]
    fn parse_empty_workspace_is_error() {
        let toml = "";
        let r: std::result::Result<OnDiskConfig, _> = toml::from_str(toml);
        assert!(r.is_err(), "missing [workspace] must fail parse");
    }

    #[test]
    fn tf_ladder_defaults_match_registry_fallback() {
        // v7.2 parity gate: the ladder the CLI/GUI derive their instance
        // defaults from must equal the registry's fallback (micro 60,
        // fast 180, slow/macro from the workspace config).
        let mut ws = WorkspaceConfig::default();
        ws.slow_timeframe.duration_seconds = 300;
        ws.macro_timeframe.duration_seconds = 900;
        let (micro, fast, slow, r#macro) = ws.tf_ladder_defaults();
        assert_eq!((micro, fast), (60, 180));
        assert_eq!((slow, r#macro), (300, 900));
    }

    #[test]
    fn execution_mode_serde_roundtrip_observe() {
        let toml = r#"
[workspace]
id = "main"
name = "Test"
default_currency = "USDC"
default_exchange = "Hyperliquid"

[[workspace.instances]]
id = "btc"
symbol = "BTC-USDT"
quote = "USDT"
mode = "observe"

[workspace.instances.micro_term]
candles = { duration_seconds = 60 }

[workspace.instances.fast_term]
candles = { duration_seconds = 180 }
"#;
        let cfg: OnDiskConfig = toml::from_str(toml).expect("observe mode must parse");
        let (_platform, workspace) = cfg.split();
        assert_eq!(workspace.instances[0].mode, ExecutionMode::Observe);

        let serialized = toml::to_string(&workspace).expect("roundtrip");
        assert!(serialized.contains("mode = \"observe\""));
    }

    #[test]
    fn default_workspace_has_zero_instances() {
        let ws = WorkspaceConfig::default();
        assert_eq!(ws.id, "main");
        assert_eq!(ws.instances.len(), 0);
        assert_eq!(ws.default_currency, "USDC");
    }

    #[test]
    fn assert_no_legacy_files_returns_ok_when_clean() {
        assert!(assert_no_legacy_files().is_ok());
    }

    #[test]
    fn declared_symbols_extracts_instance_symbols() {
        let mut ws = WorkspaceConfig::default();
        ws.instances.push(InstanceEntry {
            id: "btc".into(),
            symbol: "BTC-USDT".into(),
            quote: "USDT".into(),
            initial_capital_usd: 1000.0,
            status: InstanceStatus::Running,
            micro_term: TimeframeConfig::new(60, IndicatorsConfig::default()),
            fast_term: TimeframeConfig::new(180, IndicatorsConfig::default()),
            slow_term: None,
            macro_term: None,
            automation: AutomationConfig::default(),
            operational_mode: OperationalMode::Advisory,
            mode: ExecutionMode::default(),
            weight_overrides: None,
            position_scaling: None,
            activation: None,
            custom_pipelines: std::collections::HashMap::new(),
        });
        ws.instances.push(InstanceEntry {
            id: "eth".into(),
            symbol: "ETH-USDT".into(),
            quote: "USDT".into(),
            initial_capital_usd: 1000.0,
            status: InstanceStatus::Running,
            micro_term: TimeframeConfig::new(60, IndicatorsConfig::default()),
            fast_term: TimeframeConfig::new(180, IndicatorsConfig::default()),
            slow_term: None,
            macro_term: None,
            automation: AutomationConfig::default(),
            operational_mode: OperationalMode::Advisory,
            mode: ExecutionMode::default(),
            weight_overrides: None,
            position_scaling: None,
            activation: None,
            custom_pipelines: std::collections::HashMap::new(),
        });
        let syms = ws.declared_symbols();
        assert_eq!(syms, vec!["BTC-USDT", "ETH-USDT"]);
    }

    #[test]
    fn legacy_file_detection() {
        // Create a fake legacy file in /tmp and verify the assertion works
        // against an absolute path. We don't write to CWD here because that
        // would pollute the workspace.
        let tmp = std::env::temp_dir().join("market_monitor_legacy_test");
        std::fs::create_dir_all(&tmp).unwrap();
        let fake = tmp.join("instances.json");
        std::fs::write(&fake, "{}").unwrap();
        // We're not in `tmp`, so this assertion should succeed (no legacy
        // files in CWD).
        assert!(assert_no_legacy_files().is_ok());
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn custom_timeframes_rejected_at_load() {
        // Audit fix (M6): `custom_pipelines` is configured-but-unimplemented
        // in the runtime — the registry never instantiates custom slots. The
        // loader must fail fast instead of silently dropping the config.
        let mut ws = WorkspaceConfig::default();
        let mut inst = InstanceEntry {
            id: "btc".into(),
            symbol: "BTC-USDT".into(),
            quote: "USDT".into(),
            initial_capital_usd: 1000.0,
            status: InstanceStatus::Running,
            micro_term: TimeframeConfig::new(60, IndicatorsConfig::default()),
            fast_term: TimeframeConfig::new(180, IndicatorsConfig::default()),
            slow_term: None,
            macro_term: None,
            automation: AutomationConfig::default(),
            operational_mode: OperationalMode::Advisory,
            mode: ExecutionMode::default(),
            weight_overrides: None,
            position_scaling: None,
            activation: None,
            custom_pipelines: std::collections::HashMap::new(),
        };
        assert!(
            validate_workspace(&ws).is_ok(),
            "empty custom_pipelines must pass validation"
        );
        ws.instances.push(inst.clone());

        let mut custom = std::collections::HashMap::new();
        custom.insert(5u16, TimeframeConfig::new(120, IndicatorsConfig::default()));
        inst.custom_pipelines = custom;
        ws.instances = vec![inst];
        match validate_workspace(&ws) {
            Err(ConfigError::CustomTimeframesUnsupported {
                symbol,
                count,
                keys,
            }) => {
                assert_eq!(symbol, "BTC-USDT");
                assert_eq!(count, 1);
                assert_eq!(keys, "5");
            }
            other => panic!("expected CustomTimeframesUnsupported, got {:?}", other),
        }
    }

    #[test]
    fn zero_valued_periods_rejected_at_load() {
        // M8 (production audit): zero periods panic in the hot path
        // (Decimal/u64 division, median-window indexing) — reject at boot.
        let bad_duration = InstanceEntry {
            id: "btc".into(),
            symbol: "BTC-USDT".into(),
            quote: "USDT".into(),
            initial_capital_usd: 1000.0,
            status: InstanceStatus::Running,
            micro_term: TimeframeConfig {
                candles: CandlesConfig {
                    duration_seconds: 0,
                },
                ..TimeframeConfig::new(60, IndicatorsConfig::default())
            },
            fast_term: TimeframeConfig::new(180, IndicatorsConfig::default()),
            slow_term: None,
            macro_term: None,
            automation: AutomationConfig::default(),
            operational_mode: OperationalMode::Advisory,
            mode: ExecutionMode::default(),
            weight_overrides: None,
            position_scaling: None,
            activation: None,
            custom_pipelines: std::collections::HashMap::new(),
        };
        let mut ws = WorkspaceConfig {
            instances: vec![bad_duration],
            ..WorkspaceConfig::default()
        };
        assert!(matches!(
            validate_workspace(&ws),
            Err(ConfigError::InvalidNumeric { .. })
        ));

        let mut bad_rsi = InstanceEntry {
            id: "btc".into(),
            symbol: "BTC-USDT".into(),
            quote: "USDT".into(),
            initial_capital_usd: 1000.0,
            status: InstanceStatus::Running,
            micro_term: TimeframeConfig::new(60, IndicatorsConfig::default()),
            fast_term: TimeframeConfig::new(180, IndicatorsConfig::default()),
            slow_term: None,
            macro_term: None,
            automation: AutomationConfig::default(),
            operational_mode: OperationalMode::Advisory,
            mode: ExecutionMode::default(),
            weight_overrides: None,
            position_scaling: None,
            activation: None,
            custom_pipelines: std::collections::HashMap::new(),
        };
        bad_rsi.micro_term.indicators.rsi_period = 0;
        ws.instances = vec![bad_rsi];
        assert!(matches!(
            validate_workspace(&ws),
            Err(ConfigError::InvalidNumeric { .. })
        ));

        // Platform side: median window 0 rejected.
        let platform = PlatformConfig {
            quality: Some(QualityConfig {
                median_window_size: 0,
                ..QualityConfig::default()
            }),
            ..PlatformConfig::default()
        };
        assert!(matches!(
            validate_platform(&platform),
            Err(ConfigError::InvalidNumeric { .. })
        ));
    }

    #[test]
    fn backtest_config_defaults_and_bounds() {
        // Defaults ship valid.
        let mut ws = WorkspaceConfig::default();
        assert_eq!(ws.backtest.archive_depth_days, 180);
        assert_eq!(ws.backtest.hyperliquid.page_cap, 1000);
        assert_eq!(ws.backtest.bitget.page_cap, 200);
        assert!(validate_workspace(&ws).is_ok());

        // Depth bounds: 0 and 366 must fail, 1 and 365 must pass.
        for bad in [0u32, 366] {
            ws.backtest.archive_depth_days = bad;
            assert!(
                matches!(
                    validate_workspace(&ws),
                    Err(ConfigError::InvalidNumeric { .. })
                ),
                "depth {bad} must be rejected"
            );
        }
        for ok in [1u32, 365] {
            ws.backtest.archive_depth_days = ok;
            assert!(validate_workspace(&ws).is_ok(), "depth {ok} must pass");
        }

        // Warmup floor + page-cap sanity.
        ws.backtest.warmup_bars = 29;
        assert!(matches!(
            validate_workspace(&ws),
            Err(ConfigError::InvalidNumeric { .. })
        ));
        ws.backtest.warmup_bars = 30;
        ws.backtest.hyperliquid.page_cap = 0;
        assert!(matches!(
            validate_workspace(&ws),
            Err(ConfigError::InvalidNumeric { .. })
        ));
        ws.backtest.hyperliquid.page_cap = 1000;
        ws.backtest.bitget.max_pages_per_run = 0;
        assert!(matches!(
            validate_workspace(&ws),
            Err(ConfigError::InvalidNumeric { .. })
        ));
    }
}
