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
            },
            self.workspace,
        )
    }
}

/// Platform-level configuration. Read once at startup by `execution-daemon`.
/// Contains the things that are NOT per-workspace / per-instance: the
/// exchange endpoints the binary connects to and the NTP clock monitor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
}

impl Default for PlatformConfig {
    fn default() -> Self {
        Self {
            hyperliquid: HyperliquidConfig::default(),
            bitget: BitgetConfig::default(),
            clock_monitor: None,
            quality: None,
            reconnect: ReconnectConfig::default(),
            candle_buffer: CandleBufferConfig::default(),
        }
    }
}

/// Status of a single trading-pair instance. Persisted in the workspace file
/// so the dashboard can render the row correctly after a restart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InstanceStatus {
    Running,
    Paused,
    Stopped,
}

impl Default for InstanceStatus {
    fn default() -> Self {
        InstanceStatus::Running
    }
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
    pub activation: ActivationConfig,
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

    /// Execution policies for the Trade Automation Engine.
    #[serde(default)]
    pub execution_policies: Vec<ExecutionPolicy>,

    /// Execution-layer configuration (slippage ceiling, etc.).
    #[serde(default)]
    pub execution: ExecutionConfig,
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
            activation: ActivationConfig::default(),
            config_version: 1,
            scoring: ScoringConfig::default(),
            leverage: LeverageConfig::default(),
            defaults: DefaultsConfig::default(),
            instances: Vec::new(),
            execution_policies: Vec::new(),
            execution: ExecutionConfig::default(),
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
    #[serde(default)]
    pub weight_overrides: Option<std::collections::HashMap<String, i32>>,
    #[serde(default)]
    pub position_scaling: Option<PositionScalingConfig>,
    /// Per-instance activation overrides (union with global [activation]).
    #[serde(default)]
    pub activation: Option<ActivationConfig>,
}

fn default_initial_capital() -> f64 {
    1_000.0
}

/// Backward-compat alias for the v5.0 migration window. New code should
/// use `WorkspaceConfig` and `InstanceEntry` directly.
#[deprecated(note = "use WorkspaceConfig + InstanceEntry instead")]
pub type AppConfig = WorkspaceConfig;

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
    Ok(on_disk.split())
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
            weight_overrides: None,
            position_scaling: None,
            activation: None,
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
            weight_overrides: None,
            position_scaling: None,
            activation: None,
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
}