// engineTabs — single source of truth for the engine-level navbar rows.
//
// Every engine renders its pages through the same horizontal navbar the
// Market Monitor uses (`rowTabs` chrome in App.svelte). This module holds
// the tab list per engine plus the default (first) tab each engine lands
// on, so navigation, routing, and the store stay in sync.

export type EngineKey =
    | 'profile'
    | 'data_infra'
    | 'market_monitor'
    | 'trade_automation'
    | 'portfolio'
    | 'performance'
    | 'backtesting'
    | 'exchange_settings';

export interface EngineTab {
    key: string;
    label: string;
}

export const PROFILE_TABS: EngineTab[] = [
    { key: 'fee', label: 'Fees & Leverage' },
    { key: 'exchange', label: 'Exchange' },
    { key: 'share', label: 'Share Config' },
    { key: 'settings', label: 'Settings' },
];

export const ENGINE_TABS: Record<EngineKey, EngineTab[]> = {
    profile: PROFILE_TABS,
    exchange_settings: PROFILE_TABS,
    // v7.3: DIE tabs follow the layer order — Overview (landing) → L1 raw
    // ingestion → L2 market data → L3 data quality → L4 distribution →
    // cross-cutting (clock contract) last. v7.4: the Settings tab was
    // removed — DIE is read-only platform config; live health/quality/clock
    // data lives on Overview and the raw config.toml is exported from
    // Profile → Share Config.
    data_infra: [
        { key: 'overview', label: 'Overview' },
        { key: 'exchange_status', label: 'Exchange Status' },
        { key: 'connectivity', label: 'Connectivity' },
        { key: 'market_data', label: 'Market Data' },
        { key: 'clock_monitor', label: 'NTP Clock Monitor' },
        { key: 'data_quality', label: 'Data Quality' },
        { key: 'distribution', label: 'Distribution' },
    ],
    market_monitor: [
        { key: 'overview', label: 'Overview' },
        { key: 'workspace', label: 'Workspace' },
        { key: 'settings', label: 'Settings' },
    ],
    trade_automation: [
        { key: 'overview', label: 'Overview' },
        { key: 'orders', label: 'Orders' },
        { key: 'activity', label: 'Activity' },
        { key: 'history', label: 'Trade History' },
        { key: 'settings', label: 'Settings' },
    ],
    // v7.3: PME tabs follow L1 Position → L2 Exposure → L3 Capital →
    // L4 Portfolio, with the cross-cutting Safety ladder and Settings last.
    portfolio: [
        { key: 'overview', label: 'Overview' },
        { key: 'positions', label: 'Positions' },
        { key: 'exposure', label: 'Exposure' },
        { key: 'capital', label: 'Capital' },
        { key: 'portfolio', label: 'Portfolio' },
        { key: 'safety', label: 'Safety' },
        { key: 'settings', label: 'Settings' },
    ],
    // v7.3: PAE tabs follow L1 Trades → L2 Strategy → L3 Risk → L4
    // Performance, with cross-cutting History + Methodology + Settings
    // last. v8: the Backtesting tab moved to the Backtesting Engine.
    performance: [
        { key: 'overview', label: 'Overview' },
        { key: 'trades', label: 'Trades' },
        { key: 'strategy', label: 'Strategy' },
        { key: 'risk', label: 'Risk Metrics' },
        { key: 'performance', label: 'Performance' },
        { key: 'history', label: 'History' },
        { key: 'methodology', label: 'Methodology' },
        { key: 'settings', label: 'Settings' },
    ],
    // v8 BTE: one tab per simulated engine (DIE data → MME signals → TAE
    // executions → PME portfolio → PAE statistics), the Study Report
    // (the finished data-science presentation), History + Settings last.
    backtesting: [
        { key: 'overview', label: 'Overview' },
        { key: 'die', label: 'DIE · Data' },
        { key: 'mme', label: 'MME · Signals' },
        { key: 'tae', label: 'TAE · Executions' },
        { key: 'pme', label: 'PME · Portfolio' },
        { key: 'pae', label: 'PAE · Statistics' },
        { key: 'study', label: 'Study Report' },
        { key: 'history', label: 'History' },
        { key: 'settings', label: 'Settings' },
    ],
};

/// v8 BTE: the simplified navbar when no running instance is selected —
/// Overview (no-instance state) + History (runs are instance-independent)
/// + Settings (always present, edits [backtest]).
export const BTE_TABS_NO_INSTANCE: EngineTab[] = [
    { key: 'overview', label: 'Overview' },
    { key: 'history', label: 'History' },
    { key: 'settings', label: 'Settings' },
];

export const ENGINE_DEFAULT_TAB: Record<EngineKey, string> = {
    profile: 'settings',
    exchange_settings: 'settings',
    data_infra: 'connectivity',
    market_monitor: 'overview',
    trade_automation: 'overview',
    portfolio: 'overview',
    performance: 'overview',
    backtesting: 'overview',
};

/// v7.2: observe-mode tab collapse. An observe instance has no orders,
/// no capital, and no recorded trades — surfaces whose data source does
/// not exist are hidden so the operator only sees tabs that can answer
/// a real question. Paper and live keep the full tab set.
///
/// v7.3: the **Settings** tab is always present in every mode (per-engine
/// config, instance-independent), so observe sets are never smaller than
/// three tabs. PAE observe keeps Overview + Backtesting + History +
/// Methodology — the recorded-decision backtest is the one PAE surface
/// that works before capital is deployed.
const OBSERVE_TABS: Partial<Record<EngineKey, EngineTab[]>> = {
    trade_automation: [
        { key: 'overview', label: 'Overview' },
        { key: 'activity', label: 'Activity' },
        { key: 'settings', label: 'Settings' },
    ],
    portfolio: [
        { key: 'overview', label: 'Overview' },
        { key: 'safety', label: 'Safety' },
        { key: 'settings', label: 'Settings' },
    ],
    // v8: PAE observe keeps Overview + History + Methodology (the
    // Backtesting surface moved to the Backtesting Engine).
    performance: [
        { key: 'overview', label: 'Overview' },
        { key: 'history', label: 'History' },
        { key: 'methodology', label: 'Methodology' },
        { key: 'settings', label: 'Settings' },
    ],
};

export type ExecutionMode = 'observe' | 'paper' | 'live';

/** Resolves the tab list for an engine given the instance's execution
 *  mode. Observe collapses to the data-bearing tabs; paper/live return
 *  the full set. */
export function tabsForMode(engine: EngineKey, mode: ExecutionMode | string | undefined): EngineTab[] {
    if (mode === 'observe') {
        const collapsed = OBSERVE_TABS[engine];
        if (collapsed) return collapsed;
    }
    return ENGINE_TABS[engine];
}

/** Resolves an arbitrary `middleTab` value to a known tab of the engine,
 *  falling back to the engine's default tab. Used by the router so stale
 *  or legacy hash segments (e.g. `#/engine/data_infra/overview`) never
 *  leave the app with no active navbar item. */
export function resolveEngineTab(engine: EngineKey, middleTab: string | undefined): string {
    if (middleTab && ENGINE_TABS[engine].some((t) => t.key === middleTab)) {
        return middleTab;
    }
    return ENGINE_DEFAULT_TAB[engine];
}

/** v7.3: mode-aware section resolution. Resolves `middleTab` against the
 *  tab set the CURRENT mode actually renders (`tabsForMode`), so a stale
 *  URL like `#/engine/trade_automation/orders` in observe mode lands on
 *  the engine default instead of a phantom section — the navbar and the
 *  rendered content can never disagree. */
export function resolveEngineTabForMode(
    engine: EngineKey,
    middleTab: string | undefined,
    mode: ExecutionMode | string | undefined,
): string {
    const tabs = tabsForMode(engine, mode);
    if (middleTab && tabs.some((t) => t.key === middleTab)) {
        return middleTab;
    }
    return ENGINE_DEFAULT_TAB[engine];
}
