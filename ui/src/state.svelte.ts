// Global reactive state using Svelte 5 runes
import type {
    DecisionProfile, DecisionScore,
    RiskProfile, RiskCalculation, FeeTableRow, CommissionProjection,
    DashboardStats, TradeLedgerRecord, TradeJournalRecord,
    InstanceState, TimeframeTelemetry,
    ScaleInPortion, TakeProfitTarget, UserTrade,
    CurrentView,
    AlignmentMatrix, AnalysisMatrix, OverviewMatrix,
    ExchangeAccount,
} from './types';
import { SettingsStore } from './stores/settings.svelte';
import { AnalyticsStore } from './stores/analytics.svelte';
import { SessionStore } from './stores/session.svelte';
import { ProfileStore } from './stores/profiles.svelte';

function createTimeframeTelemetry(
    symbol: string,
    slot: 'micro' | 'fast' | 'slow' | 'macro',
    barDurationSec: number,
): TimeframeTelemetry {
    return {
        slot,
        symbol, exchange: 'Hyperliquid', barDurationSec,
        indicators: {},
        priceText: '--', volText: '--', avgVolText: '--',
        showPatterns: true,
        isCompleted: false, latestSnapshot: null, historyPrices: [],
        showEmas: true, showBb: true, showVwap: true, showVolume: true,
        showAdx: true, showAtr: true, showRsi: true, showMacd: true,
        showSqueeze: true, showBbwp: true, showFib: true, showRvol: true,
        showStochastic: true, showChandeMo: true,
        showSupertrend: true, showKeltner: true, showDonchian: true,
        showIchimoku: true, showHullMa: true, showPsar: true, showStddevChan: true,
        showObv: true, showCmf: true, showMfi: true, showHv: true,
        showAroon: true, showChoppiness: true, showLinregSlope: true, showZscore: true,
        showLiqHeatmap: false, showVolumeProfile: false,
        showWilliamsR: false, showCci: false, showForceIdx: false,
        showFunding: false, showOpenInterest: false, showOiDelta: false,
        showOrderFlowDepth: false, showDerivativeRibbon: true,
        showPivotPoints: false, showSupportResistance: false,
        showSmcStructure: false, showSmcLiquidity: false,
        showFvgZones: false, showOrderBlocks: false,
        showAnchoredVwap: false,
        emaFastVal: 10, emaMediumVal: 50, emaSlowVal: 100, emaLongVal: 200,
        rsiPeriodVal: 14, macdFastVal: 12, macdSlowVal: 26, macdSignalVal: 9,
        adxPeriodVal: 14, atrPeriodVal: 14, squeezePeriodVal: 20,
        bbwpPeriodVal: 20, bbwpLookbackVal: 252, analysisLimit: 100,
        stochKPeriodVal: 18, stochDPeriodVal: 5, stochSPeriodVal: 9, chandemoPeriodVal: 12,
        supertrendPeriodVal: 10, supertrendMultiplierVal: 3.0,
        keltnerEmaPeriodVal: 20, keltnerAtrPeriodVal: 10, keltnerMultiplierVal: 2.0,
        donchianPeriodVal: 20, obvSmoothingVal: 20, cmfPeriodVal: 20, mfiPeriodVal: 14, hvPeriodVal: 20,
        aroonPeriodVal: 25, chopPeriodVal: 14, linregPeriodVal: 20, zscorePeriodVal: 20,
        macdExtremeHighVal: 1000, macdExtremeLowVal: -1000, macdContractionVal: 0.30,
        adxTrendThresholdVal: 20, adxExhaustionThresholdVal: 40, adxSlopeLookbackVal: 3,
        squeezeMinDurationVal: 5, squeezeBbPeriodVal: 20, squeezeBbStdDevVal: 2.0,
        squeezeKcPeriodVal: 20, squeezeKcAtrMultVal: 1.5,
        atrMultiplierVal: 2.0, atrTargetRRVal: 2.5,
        volumeAvgPeriodVal: 20, rvolInstitutionalVal: 1.5, rvolClimaxVal: 3.0,
    };
}

function createInstanceState(symbol: string): InstanceState {
    return {
        symbol, exchange: 'Hyperliquid', isConnected: false,
        microTerm: createTimeframeTelemetry(symbol, 'micro', 60),
        fastTerm: createTimeframeTelemetry(symbol, 'fast', 180),
        slowTerm: createTimeframeTelemetry(symbol, 'slow', 300),
        macroTerm: createTimeframeTelemetry(symbol, 'macro', 900),
        historyLatestClose: '0',
        currentView: 'terminal',
        alignment: null,
        analysis: null,
        risk: null,
        advisory: null,
        automationEnabled: false,
        automationIntervalMode: 'interval',
        automationIntervalValue: 900,
        automationIntervalUnit: 'seconds',
        priceLineMode: false,
        slowIntervalSecs: 900,
        normalIntervalSecs: 300,
        fastIntervalSecs: 60,
        showEmaFast: false,
        showEmaMedium: false,
        showEmaSlow: false,
        showEmaLong: false,
    };
}

export class AppStore {
    // ─── Sub-stores ───────────────────────────────────────────────────
    settings = new SettingsStore();
    analytics = new AnalyticsStore();
    session = new SessionStore();
    profiles = new ProfileStore();

    // ─── Global State ─────────────────────────────────────────────────
    instancesMap = $state<Record<string, InstanceState>>({});
    activeTab = $state<string>('BTC-USDT');
    /// Bumped by `bumpWsVersion()` after every config save so the
    /// reconnect `$effect` in `App.svelte` re-runs and re-attaches each
    /// WS connection with the new `timeframe_secs`.
    wsVersion = $state(0);
    currentGlobalView = $state<string>('dashboard');
    overviewMatrix = $state<OverviewMatrix | null>(null);

    // ─── Fullscreen chart modal ───────────────────────────────────────
    // Rendered at the root of App.svelte so it escapes the grid container's
    // stacking context and covers the entire viewport (including the top
    // navigation bar). null = modal closed.
    fullscreenChart = $state<{ chartType: string; slot: 'micro' | 'fast' | 'slow' | 'macro'; pairKey: string } | null>(null);
    openFullscreenChart(chartType: string, slot: 'micro' | 'fast' | 'slow' | 'macro', pairKey: string) {
        this.fullscreenChart = { chartType, slot, pairKey };
    }
    closeFullscreenChart() {
        this.fullscreenChart = null;
    }

    // ─── Grid cockpit navigation state ────────────────────────────────
    isManageModalOpen = $state(false);
    currentEngine = $state<'data_infra' | 'market_monitor' | 'portfolio' | 'trade_automation' | 'performance' | 'profile' | 'exchange_settings'>('profile');
    middleTab = $state<string>('overview');
    activeEngineTab = $state<'overview' | 'instance'>('overview');
    selectedInstance = $state<string | null>(null);

    selectEngine(engine: 'data_infra' | 'market_monitor' | 'portfolio' | 'trade_automation' | 'performance' | 'profile' | 'exchange_settings') {
        this.currentEngine = engine;
        this.middleTab = engine === 'market_monitor' ? 'overview' : 'overview';
        if (engine === 'market_monitor') {
            this.activeEngineTab = this.selectedInstance ? 'instance' : 'overview';
        }
    }

    enterInstance(pairKey: string) {
        const base = pairKey.includes('-') ? pairKey.split('-')[0] : pairKey;
        if (!this.instancesMap[pairKey]) this.initInstance(base);
        this.selectedInstance = pairKey;
        this.activeTab = pairKey;
        this.currentEngine = 'market_monitor';
        this.activeEngineTab = 'instance';
        const pair = this.instancesMap[pairKey];
        if (pair) pair.currentView = 'terminal';
    }

    exitInstance() {
        this.selectedInstance = null;
        this.activeEngineTab = 'overview';
    }

    // ─── Paper Trading State ──────────────────────────────────────────
    paperLoading = $state(false);
    paperCashBalance = $state(0);
    paperMarginUsed = $state(0);
    paperTotalAccountValue = $state(0);
    paperDirection = $state('');
    paperLeverage = $state(1);
    paperUnrealizedPnl = $state(0);
    paperUnrealizedRoi = $state(0);
    paperInitialUSD = $state(10000);
    paperAllocationPct = $state(20);
    paperAutoExecute = $state(false);
    paperBreakEvenTrailEnabled = $state(false);
    activePaperPosition = $state<Record<string, unknown> | null>(null);
    paperHistory = $state<Record<string, unknown>[]>([]);
    openOrders = $state<Record<string, unknown>[]>([]);
    activeSlots = $state<Record<string, unknown>[]>([]);
    activeEntryOrders = $state<Record<string, unknown>[]>([]);
    positionBrackets = $state<Record<string, unknown>[]>([]);
    paper = {
        openOrders: [] as Record<string, unknown>[],
    };

    async fetchPaperStatus() { /***/ }
    async fetchOpenOrders() { /***/ }
    async cancelOrder(_orderId: unknown) { /***/ }
    async setTpTargets(_targets: unknown[]) { /***/ }
    async setSlLevels(_levels: unknown[]) { /***/ }
    async closePositionPct(_pct: number) { return { success: false, message: '' }; }
    async savePaperConfig(_initialUSD: number, _allocPct: number, _autoExec: boolean) { /***/ }

    exchangeAccounts = $state<ExchangeAccount[]>([]);
    exchangeActiveCount = $state(0);
    exchangeMaxAccounts = $state(5);
    exchangeFormDraft = $state({
        exchange: 'Hyperliquid', account_name: '', api_key: '',
        api_secret: '', passphrase: '', referred_uid: '', is_active: true,
    });
    async fetchExchangeKeys() { /***/ }
    async addExchangeKey() { /***/ }
    async deleteExchangeKey(_id: number) { /***/ }

    // ─── Legacy State ─────────────────────────────────────────────────
    _currentPosition = $state<string>('None');
    get currentPosition(): string { return this._currentPosition; }
    set currentPosition(v: string) {
        this._currentPosition = v;
        if (v === 'None') this.entryPriceVal = '';
    }
    entryPriceVal = $state<string>('');
    analysisPhase = $state<string>('idle');
    currentLevel2Mode = $state<string>('user');
    _modeViews: Record<string, string> = {};
    _lastMode: string = '';

    constructor() {
        this.session.onSessionActivated = () => {
            this.currentEngine = 'profile';
            this.activeEngineTab = 'overview';
            this.selectedInstance = null;
        };

        this._delegate(this.session, [
            'sessionActive', 'sessionCurrency', 'sessionExchange',
            'sessionCapital', 'sessionInstanceCount',
            'sessionLoading', 'sessionChecked', 'sessionError',
        ]);

        this._delegate(this.profiles, [
            'activeDecisionProfileId', 'decisionProfiles', 'calculatedDecisionScore',
            'decisionLoading', 'activeRiskProfileId', 'riskProfiles', 'riskDirection',
            'riskEntryPrice', 'riskStopLoss', 'riskTakeProfit', 'riskCalculation',
            'riskCalculating', 'useDynamicAtr', 'atrValue',
            'commissionDirection', 'commissionEntry1', 'commissionEntry2',
            'commissionSL1', 'commissionSL2', 'commissionTP1', 'commissionTP2',
            'commissionCapitalSplit', 'commissionOrderType', 'commissionProjection',
            'commissionLoading', 'feeTable', 'feeTableLoading',
        ]);

        this._delegate(this.settings, [
            'apiKeyConfigured', 'rulesContent', 'globalCandlesConfig',
            'globalIndicatorsConfig', 'indicatorRegistry', 'emaFastLabel', 'emaMediumLabel',
            'emaSlowLabel', 'emaLongLabel', 'rsiLabel', 'adxLabel', 'atrLabel',
            'macdLabel',
        ]);

        this._delegate(this.analytics, [
            'dashboardStats', 'tradeLedgerRecords', 'tradeJournalRecords',
        ]);

        this._delegateMethods(this.session, 'fetchSessionStatus', 'initSession', 'quitSession');
        this._delegateMethods(this.profiles,
            'fetchDecisionProfiles', 'createDecisionProfile', 'deleteDecisionProfile',
            'updateDecisionProfileThresholds', 'addProfileIndicator',
            'updateProfileIndicator', 'deleteProfileIndicator',
            'fetchRiskProfiles', 'createRiskProfile', 'deleteRiskProfile',
            'calculateRisk', 'fetchFeeTable', 'calculateCommissionProjection');
        this._delegateMethods(this.analytics,
            'fetchTradeLedger', 'fetchTradeJournal', 'updateJournalNotes',
            'fetchDashboardStats');
    }

    switchMode(mode: string) {
        const modes: Record<string, { l2: string }> = {
            general: { l2: 'general' },
            user: { l2: 'user' },
            rule: { l2: 'rule' },
        };
        const defaultViews: Record<string, string> = {
            user: 'positions',
            rule: 'rule',
            general: 'ledger',
        };
        if (this._lastMode) {
            this._modeViews[this._lastMode] = this.currentView;
        }
        const m = modes[mode];
        if (m) {
            this.currentLevel2Mode = m.l2;
            this.currentView = (this._modeViews[mode] ?? (defaultViews[mode] ?? 'terminal')) as CurrentView;
            this._lastMode = mode;
        }
    }

    private _delegate(target: any, props: string[]) {
        for (const prop of props) {
            Object.defineProperty(this, prop, {
                get() { return target[prop as keyof typeof target]; },
                set(v: any) { (target as any)[prop] = v; },
                enumerable: true,
                configurable: true,
            });
        }
    }

    private _delegateMethods(target: any, ...methods: string[]) {
        for (const method of methods) {
            (this as any)[method] = (...args: any[]) => target[method](...args);
        }
    }

    async evaluateDecision(profileId: number) {
        const pair = this.activeInstance();
        await this.profiles.evaluateDecision(profileId, this.activeTab, pair.microTerm.latestSnapshot);
    }

    // ─── Helpers ─────────────────────────────────────────────────────

    activeInstance(): InstanceState {
        if (!this.instancesMap[this.activeTab]) {
            this.instancesMap[this.activeTab] = createInstanceState(this.activeTab.split('-')[0] || 'BTC');
        }
        return this.instancesMap[this.activeTab];
    }

    micro(): TimeframeTelemetry { return this.activeInstance().microTerm; }

    // ─── Quote-asset abstraction ─────────────────────────────────────
    get quote(): string { return this.sessionCurrency || 'USDT'; }
    pairKeyFor(symbol: string): string { return `${symbol}-${this.quote}`; }
    pairDisplayFor(symbol: string): string { return `${symbol}/${this.quote}`; }

    initInstance(symbol: string, _exchange?: string, instanceId?: string) {
        const key = this.pairKeyFor(symbol);
        if (!this.instancesMap[key]) {
            const created = createInstanceState(symbol);
            if (instanceId) created.instanceId = instanceId;
            this.instancesMap[key] = created;
        } else {
            if (instanceId && !this.instancesMap[key].instanceId) {
                this.instancesMap[key].instanceId = instanceId;
            }
            const pair = this.instancesMap[key];
            for (const tf of [pair.microTerm, pair.fastTerm, pair.slowTerm, pair.macroTerm] as TimeframeTelemetry[]) {
                tf.emaFastVal = this.settings.globalIndicatorsConfig.ema_fast;
                tf.emaMediumVal = this.settings.globalIndicatorsConfig.ema_medium;
                tf.emaSlowVal = this.settings.globalIndicatorsConfig.ema_slow;
                tf.emaLongVal = this.settings.globalIndicatorsConfig.ema_long;
                tf.rsiPeriodVal = this.settings.globalIndicatorsConfig.rsi_period;
                tf.macdFastVal = this.settings.globalIndicatorsConfig.macd_fast;
                tf.macdSlowVal = this.settings.globalIndicatorsConfig.macd_slow;
                tf.macdSignalVal = this.settings.globalIndicatorsConfig.macd_signal;
                tf.adxPeriodVal = this.settings.globalIndicatorsConfig.adx_period;
                tf.atrPeriodVal = this.settings.globalIndicatorsConfig.atr_period;
                tf.squeezePeriodVal = this.settings.globalIndicatorsConfig.squeeze_period;
                tf.stochKPeriodVal = this.settings.globalIndicatorsConfig.stoch_k_period ?? 18;
                tf.stochDPeriodVal = this.settings.globalIndicatorsConfig.stoch_d_period ?? 5;
                tf.stochSPeriodVal = this.settings.globalIndicatorsConfig.stoch_s_period ?? 9;
                tf.chandemoPeriodVal = this.settings.globalIndicatorsConfig.chandemo_period ?? 12;
                tf.analysisLimit = this.settings.globalCandlesConfig.analysis_limit ?? 100;
            }
        }
    }

    removeInstance(key: string) { delete this.instancesMap[key]; }

    switchTab(key: string) { this.activeTab = key; }

    /// Signal the WS reconnect effect in `App.svelte` to tear down and
    /// re-attach all WebSocket connections with the current per-slot
    /// durations. Must be called after every save in `WorkspaceSettings`
    /// and `TimeframeSettings` so the WS URL's `timeframe_secs` matches
    /// the new pipeline's `barDurationSec`.
    bumpWsVersion(): void { this.wsVersion++; }

    // ─── Instance / Telemetry Accessors ──────────────────────────────

    get microTerm() { return this.activeInstance().microTerm; }
    get fastTerm() { return this.activeInstance().fastTerm; }
    get slowTerm() { return this.activeInstance().slowTerm; }
    get macroTerm() { return this.activeInstance().macroTerm; }
    get activeSymbol() { return this.activeInstance().symbol; }
    get activeExchange() { return this.activeInstance().exchange; }
    get isConnected() { return this.activeInstance().isConnected; }
    set isConnected(v: boolean) { this.activeInstance().isConnected = v; }

    // Micro-term telemetry accessors
    get priceText() { return this.micro().priceText; }
    set priceText(v: string) { this.micro().priceText = v; }
    get avgVolText() { return this.micro().avgVolText; }
    set avgVolText(v: string) { this.micro().avgVolText = v; }
    get volText() { return this.micro().volText; }
    set volText(v: string) { this.micro().volText = v; }
    get latestSnapshot() { return this.micro().latestSnapshot; }
    set latestSnapshot(v: Record<string, unknown> | null) { this.micro().latestSnapshot = v; }
    get historyPrices() { return this.micro().historyPrices; }
    set historyPrices(v: number[]) { this.micro().historyPrices = v; }

    get showEmas() { return this.micro().showEmas; }
    set showEmas(v: boolean) { this.micro().showEmas = v; }
    get showBb() { return this.micro().showBb; }
    set showBb(v: boolean) { this.micro().showBb = v; }
    get showVwap() { return this.micro().showVwap; }
    set showVwap(v: boolean) { this.micro().showVwap = v; }
    get showVolume() { return this.micro().showVolume; }
    set showVolume(v: boolean) { this.micro().showVolume = v; }
    get showAdx() { return this.micro().showAdx; }
    set showAdx(v: boolean) { this.micro().showAdx = v; }
    get showAtr() { return this.micro().showAtr; }
    set showAtr(v: boolean) { this.micro().showAtr = v; }
    get showRsi() { return this.micro().showRsi; }
    set showRsi(v: boolean) { this.micro().showRsi = v; }
    get showMacd() { return this.micro().showMacd; }
    set showMacd(v: boolean) { this.micro().showMacd = v; }
    get showSqueeze() { return this.micro().showSqueeze; }
    set showSqueeze(v: boolean) { this.micro().showSqueeze = v; }

    get barDurationSec() { return this.micro().barDurationSec; }
    set barDurationSec(v: number) { this.micro().barDurationSec = v; }
    get emaFastVal() { return this.micro().emaFastVal; }
    set emaFastVal(v: number) { this.micro().emaFastVal = v; }
    get emaMediumVal() { return this.micro().emaMediumVal; }
    set emaMediumVal(v: number) { this.micro().emaMediumVal = v; }
    get emaSlowVal() { return this.micro().emaSlowVal; }
    set emaSlowVal(v: number) { this.micro().emaSlowVal = v; }
    get emaLongVal() { return this.micro().emaLongVal; }
    set emaLongVal(v: number) { this.micro().emaLongVal = v; }
    get rsiPeriodVal() { return this.micro().rsiPeriodVal; }
    set rsiPeriodVal(v: number) { this.micro().rsiPeriodVal = v; }
    get macdFastVal() { return this.micro().macdFastVal; }
    set macdFastVal(v: number) { this.micro().macdFastVal = v; }
    get macdSlowVal() { return this.micro().macdSlowVal; }
    set macdSlowVal(v: number) { this.micro().macdSlowVal = v; }
    get macdSignalVal() { return this.micro().macdSignalVal; }
    set macdSignalVal(v: number) { this.micro().macdSignalVal = v; }
    get adxPeriodVal() { return this.micro().adxPeriodVal; }
    set adxPeriodVal(v: number) { this.micro().adxPeriodVal = v; }
    get atrPeriodVal() { return this.micro().atrPeriodVal; }
    set atrPeriodVal(v: number) { this.micro().atrPeriodVal = v; }
    get squeezePeriodVal() { return this.micro().squeezePeriodVal; }
    set squeezePeriodVal(v: number) { this.micro().squeezePeriodVal = v; }
    get analysisLimit() { return this.micro().analysisLimit; }
    set analysisLimit(v: number) { this.micro().analysisLimit = v; }

    get historyLatestClose() { return this.activeInstance().historyLatestClose; }
    set historyLatestClose(v: string) { this.activeInstance().historyLatestClose = v; }

    get currentView() { return this.activeInstance().currentView; }
    set currentView(v: CurrentView) { this.activeInstance().currentView = v; }
}

// Module-level singleton for backward compatibility
const store = new AppStore();

export function useAppStore(): AppStore {
    return store;
}

export function createAppStore(): AppStore {
    return new AppStore();
}
