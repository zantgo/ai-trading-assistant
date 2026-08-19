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
    | 'exchange_settings';

export interface EngineTab {
    key: string;
    label: string;
}

export const PROFILE_TABS: EngineTab[] = [
    { key: 'fee', label: 'Fee Projection' },
    { key: 'exchange', label: 'Exchange' },
    { key: 'share', label: 'Share Config' },
    { key: 'settings', label: 'Settings' },
];

export const ENGINE_TABS: Record<EngineKey, EngineTab[]> = {
    profile: PROFILE_TABS,
    exchange_settings: PROFILE_TABS,
    data_infra: [
        { key: 'connectivity', label: 'Connectivity' },
        { key: 'exchange_status', label: 'Exchange Status' },
        { key: 'clock_monitor', label: 'NTP Clock Monitor' },
        { key: 'data_quality', label: 'Data Quality' },
        { key: 'settings', label: 'Settings' },
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
    ],
    portfolio: [
        { key: 'overview', label: 'Overview' },
        { key: 'positions', label: 'Positions' },
        { key: 'exposure', label: 'Exposure' },
        { key: 'capital', label: 'Capital' },
        { key: 'safety', label: 'Safety' },
    ],
    performance: [
        { key: 'overview', label: 'Overview' },
        { key: 'strategy', label: 'Strategy' },
        { key: 'risk', label: 'Risk Metrics' },
        { key: 'regimes', label: 'Regime Map' },
        { key: 'trades', label: 'Trade Analytics' },
        { key: 'backtesting', label: 'Backtesting' },
    ],
};

export const ENGINE_DEFAULT_TAB: Record<EngineKey, string> = {
    profile: 'settings',
    exchange_settings: 'settings',
    data_infra: 'connectivity',
    market_monitor: 'overview',
    trade_automation: 'overview',
    portfolio: 'overview',
    performance: 'overview',
};

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
