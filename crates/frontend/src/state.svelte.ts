// Global reactive state using Svelte 5 runes
import type {
    AssistantAnalysis, AssistantHistoryRecord, ChatMessage, DecisionProfile, DecisionScore,
    RiskProfile, RiskCalculation, FeeTableRow, CommissionProjection, ExchangeAccount,
    DashboardStats, TradeLedgerRecord, TradeJournalRecord,
    WizardAnalysisResponse, InstanceState, TimeframeTelemetry,
    ScaleInPortion, TakeProfitTarget, UserTrade,
    SystemHeartbeat, DecisionMemoryRow, CompletedTradesRow,
    CurrentView, Level2Mode, OperationalMode,
} from './types';
import { PaperTradingStore } from './stores/paperTrading.svelte';
import { SettingsStore } from './stores/settings.svelte';
import { AnalyticsStore } from './stores/analytics.svelte';
import { SessionStore } from './stores/session.svelte';
import { ProfileStore } from './stores/profiles.svelte';
import { ExchangeKeyStore } from './stores/exchangeKeys.svelte';
import { useEdgeStore, type EdgeStore } from './stores/edges.svelte';

/** Maps a Level 2 paradigm to its backend operational_mode (general leaves it unchanged). */
const MODE_TO_OP: Record<Level2Mode, OperationalMode | null> = {
    general: null,
    wizard: null,
    risk: null,
    user: 'ManualOnly',
    rule: 'DeterministicHeuristics',
    ai: 'HybridAiCopilot',
};

function createTimeframeTelemetry(symbol: string, barDurationSec: number): TimeframeTelemetry {
    return {
        symbol, exchange: 'Hyperliquid', barDurationSec,
        indicators: {},
        priceText: '--', volText: '--', avgVolText: '--',
        prevDayPx: null,
        showPatterns: true,
        isCompleted: false, latestSnapshot: null, decisionContext: null, statisticalContext: null, historyPrices: [],
        showEmas: true, showBb: true, showVwap: true, showAvwap: true, showVolume: true,
        showAdx: true, showAtr: true, showRsi: true, showMacd: true,
        showSqueeze: true, showBbwp: true, showFib: true, showRvol: true,
        showPivots: true, showCandlestick: true, showIchimoku: true, showChikou: false, showIchimokuCloud: true,
        showStochastic: true, showChandeMo: true,
        showSupertrend: true, showKeltner: true, showDonchian: true,
        showObv: true, showCmf: true, showMfi: true, showHv: true,
        showAroon: true, showChoppiness: true, showLinregSlope: true, showZscore: true,
        showCci: true, showPsar: true,
        showWilliamsR: true, showHullMa: true, showAo: true, showForceIdx: true, showStdDevChnl: true,
        showVolumeProfile: true,
        showSmcStructure: true,
        showSmcLiquidity: false,
        showSmcFvg: false,
        showSmcOrderBlocks: true,
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
        williamsRPeriodVal: 14, hullMaPeriodVal: 16, stddevChnlPeriodVal: 20, forceIdxSmoothingVal: 13,
    };
}

function createInstanceState(symbol: string): InstanceState {
    return {
        symbol, exchange: 'Hyperliquid', isConnected: false,
        microTerm: createTimeframeTelemetry(symbol, 60),
        fastTerm: createTimeframeTelemetry(symbol, 180),
        slowTerm: createTimeframeTelemetry(symbol, 300),
        macroTerm: createTimeframeTelemetry(symbol, 900),
        assistantHistory: [], chatHistory: [],
        currentPosition: 'None', entryPriceVal: '', stopLossVal: '',
        assistantLoading: false, assistantError: null,
        assistantResponse: null, wizardResponse: null,
        analysisPhase: 'idle',
        historyLatestClose: '0',
        isAssistantModalOpen: false, chatInputText: '', isChatLoading: false,
        currentView: 'terminal',
        currentLevel2Mode: 'user',
        modeViews: { general: 'terminal', wizard: 'workflow', risk: 'risk_overview', user: 'terminal', rule: 'decision', ai: 'assistant' },
        activeExecutionMode: 'HybridAiCopilot',
        automationEnabled: false, automationIntervalValue: 15,
        automationIntervalUnit: 'minutes',
        slowIntervalSecs: 3600, normalIntervalSecs: 900, fastIntervalSecs: 300,
        nextEvaluationIn: '--',
        totalPointsScore: 0, allocatedCapitalPct: 0, activeOppositeSignalsCount: 0,
        markedSupportLevels: [], markedResistanceLevels: [], srFlipEvents: '[]',
        priceLineMode: false,
        showEmaFast: true, showEmaMedium: true, showEmaSlow: true, showEmaLong: true,
        indicatorWeights: {},
    };
}

export class AppStore {
    // ─── Sub-stores ───────────────────────────────────────────────────
    paper = new PaperTradingStore();
    settings = new SettingsStore();
    analytics = new AnalyticsStore();
    session = new SessionStore();
    profiles = new ProfileStore();
    exchangeKeys = new ExchangeKeyStore();
    edges: EdgeStore;

    // ─── Global State ─────────────────────────────────────────────────
    instancesMap = $state<Record<string, InstanceState>>({});
    activeTab = $state<string>('BTC-USDT');
    currentGlobalView = $state<string>('dashboard');
    showQuitDialog = $state(false);

    // ─── Institutional Risk Management Layer ──────────────────────────
    riskProfile = $state<import('./types').RiskProfileResponse | null>(null);
    riskProfileLoading = $state(false);
    riskProfileError = $state<string | null>(null);

    // Delegated from SessionStore (declared for TypeScript)
    declare sessionUserName: string;
    declare sessionWalletAddress: string;
    declare saveProfile: (userName: string, walletAddress: string) => Promise<boolean>;
    declare fetchProfile: () => Promise<{ userName: string; walletAddress: string } | null>;

    constructor() {
        this.edges = useEdgeStore();
        this.session.onSessionActivated = () => { this.currentGlobalView = 'dashboard'; };

        this._delegate(this.session, [
            'sessionActive', 'sessionMode', 'sessionCurrency', 'sessionExchange',
            'sessionCapital', 'sessionInstanceCount', 'sessionMaxInstances',
            'sessionLoading', 'sessionChecked', 'sessionError',
            'sessionUserName', 'sessionWalletAddress',
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

        this._delegate(this.exchangeKeys, [
            'exchangeAccounts', 'exchangeActiveCount', 'exchangeMaxAccounts', 'exchangeFormDraft',
        ]);

        this._delegate(this.paper, [
            'paperCashBalance', 'paperInitialUSD', 'paperAllocationPct',
            'paperAutoExecute', 'activePaperPosition', 'paperUnrealizedPnl',
            'paperUnrealizedRoi', 'paperTotalAccountValue', 'paperMarginUsed',
            'paperMaxTrades', 'paperActiveTrades', 'paperAvailableTrades',
            'paperHistory', 'paperLoading', 'paperScaleInPortions',
            'paperTakeProfitTargets', 'paperAvgEntryPrice',
            'paperInvalidationLevel', 'paperFilledPortions', 'paperMaxRiskPct',
            'paperLeverage', 'paperAutoExecuteIntervals', 'paperLookbackTrades',
            'paperBreakEvenTrailEnabled',
            'paperPositionPct', 'paperFreeBalancePct', 'paperDirection',
            'openOrders',
            'activeSlots', 'positionSlots', 'equitySnapshots',
            'paperInitialAllocatedMargin', 'paperRealizedPnlAccumulator',
        ]);

        this._delegate(this.settings, [
            'apiKeyConfigured', 'rulesContent', 'globalCandlesConfig',
            'globalIndicatorsConfig', 'indicatorRegistry', 'emaFastLabel', 'emaMediumLabel',
            'emaSlowLabel', 'emaLongLabel', 'rsiLabel', 'adxLabel', 'atrLabel',
            'macdLabel',
            'costPriceInput', 'costPriceOutput', 'costIntervalSecs',
            'costRunsPerDay', 'costTokensPerRunInput', 'costTokensPerRunOutput',
            'costDailyProjected', 'costWeeklyProjected', 'costMonthlyProjected',
            'costActualInputTokens', 'costActualOutputTokens', 'costActualTotal',
            'costLoading',
        ]);

        this._delegate(this.analytics, [
            'dashboardStats', 'dashboardActiveFilter', 'dashboardPeriod',
            'dashboardOrigin', 'tradeLedgerRecords', 'tradeJournalRecords',
            'journalLookbackDepth', 'systemHeartbeat', 'recentDecisions',
            'completedTrades', 'userTrades',
        ]);

        this._delegateMethods(this.session, 'fetchSessionStatus', 'quitSession', 'saveProfile', 'fetchProfile');
        this._delegateMethods(this.profiles,
            'fetchDecisionProfiles', 'createDecisionProfile', 'deleteDecisionProfile',
            'updateDecisionProfileThresholds', 'addProfileIndicator',
            'updateProfileIndicator', 'deleteProfileIndicator',
            'fetchRiskProfiles', 'createRiskProfile', 'deleteRiskProfile',
            'calculateRisk', 'fetchFeeTable', 'calculateCommissionProjection');
        this._delegateMethods(this.exchangeKeys, 'fetchExchangeKeys', 'addExchangeKey', 'deleteExchangeKey');
        this._delegateMethods(this.analytics,
            'fetchTradeLedger', 'fetchTradeJournal', 'updateJournalNotes',
            'fetchSystemStatus', 'fetchObservabilityBuffers',
            'fetchTrades', 'exportJournalCSV', 'exportJournalJSON');
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

    async initSession(mode: string, currency: string, exchange: string, capital: number, userName: string): Promise<{ success: boolean; error?: string }> {
        return this.session.initSession(mode, currency, exchange, capital, userName);
    }

    activeInstance(): InstanceState {
        if (!this.instancesMap[this.activeTab]) {
            this.instancesMap[this.activeTab] = createInstanceState(this.activeTab.split('-')[0] || 'BTC');
        }
        return this.instancesMap[this.activeTab];
    }

    micro(): TimeframeTelemetry { return this.activeInstance().microTerm; }

    // ─── Quote-asset abstraction ─────────────────────────────────────
    // The settlement/quote currency is decided at the Welcome Gate and drives
    // every pair key ("BASE-<quote>") and display label ("BASE/<quote>").
    get quote(): string { return this.sessionCurrency || 'USDT'; }
    pairKeyFor(symbol: string): string { return `${symbol}-${this.quote}`; }
    pairDisplayFor(symbol: string): string { return `${symbol}/${this.quote}`; }

    /** Per-instance capital when portfolio is divided by max instances */
    get perInstanceCapital(): number {
        if (!this.sessionCapital || !this.sessionMaxInstances || this.sessionMaxInstances === 0) return 0;
        return this.sessionCapital / this.sessionMaxInstances;
    }

    initInstance(symbol: string, _exchange?: string) {
        const key = this.pairKeyFor(symbol);
        if (!this.instancesMap[key]) {
            this.instancesMap[key] = createInstanceState(symbol);
        } else {
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
                tf.williamsRPeriodVal = this.settings.globalIndicatorsConfig.williams_r_period ?? 14;
                tf.hullMaPeriodVal = this.settings.globalIndicatorsConfig.hull_ma_period ?? 16;
                tf.stddevChnlPeriodVal = this.settings.globalIndicatorsConfig.stddev_channel_period ?? 20;
                tf.forceIdxSmoothingVal = this.settings.globalIndicatorsConfig.force_index_smoothing ?? 13;
                tf.analysisLimit = this.settings.globalCandlesConfig.analysis_limit ?? 100;
            }
        }
    }

    removeInstance(key: string) { delete this.instancesMap[key]; }

    switchTab(key: string) { this.activeTab = key; }

    autoLogTrade(pair: InstanceState, oldPosition: 'Long' | 'Short') {
        const entryPrice = parseFloat(pair.entryPriceVal);
        const exitPrice = parseFloat(pair.microTerm.priceText);
        if (isNaN(entryPrice) || isNaN(exitPrice) || entryPrice <= 0 || exitPrice <= 0) {
            console.warn("⚠️ Trade Logger Bypassed: Entry Price or Current Market Price is invalid.");
            return;
        }
        const stopLoss = parseFloat(pair.stopLossVal);
        let riskDistance = 0;
        if (!isNaN(stopLoss) && stopLoss > 0 && stopLoss !== entryPrice) {
            riskDistance = Math.abs(entryPrice - stopLoss);
        } else {
            riskDistance = entryPrice * 0.01;
        }
        let pnl = 0;
        if (oldPosition === 'Long') pnl = exitPrice - entryPrice;
        else pnl = entryPrice - exitPrice;
        const outcome = pnl >= 0 ? 'WIN' : 'LOSS';
        const rewardDistance = Math.abs(pnl);
        const rewardMultiplier = riskDistance > 0 ? (rewardDistance / riskDistance) : 1.0;
        const payload = {
            symbol: pair.symbol.toUpperCase(), direction: oldPosition, outcome,
            risk_multiplier: 1.0, reward_multiplier: parseFloat(rewardMultiplier.toFixed(2)),
        };
        fetch('/api/trades', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(payload) })
            .then(res => {
                if (res.ok) {
                    console.log(`✅ Auto-Logged Trade: ${payload.symbol} ${payload.direction} ${payload.outcome}`);
                    this.analytics.fetchTrades().catch(() => {});
                }
            }).catch(err => console.error("❌ Auto-Logger Network Error:", err));
    }

    // ─── Instance / Telemetry Accessors ──────────────────────────────

    get microTerm() { return this.activeInstance().microTerm; }
    get fastTerm() { return this.activeInstance().fastTerm; }
    get slowTerm() { return this.activeInstance().slowTerm; }
    get macroTerm() { return this.activeInstance().macroTerm; }
    get activeSymbol() { return this.activeInstance().symbol; }
    get activeExchange() { return this.activeInstance().exchange; }
    get isConnected() { return this.activeInstance().isConnected; }
    set isConnected(v: boolean) { this.activeInstance().isConnected = v; }

    // Micro-term telemetry accessors (core market data only; indicators live
    // in the nested `indicators` map on each timeframe).
    get priceText() { return this.micro().priceText; }
    set priceText(v: string) { this.micro().priceText = v; }
    get prevDayPx() { return this.micro().prevDayPx; }
    set prevDayPx(v: number | null) { this.micro().prevDayPx = v; }
    /** 24h change % derived from the current mark price vs prior-day price. */
    get dayChangePct(): number | null {
        const prev = this.micro().prevDayPx;
        const mid = parseFloat(this.micro().priceText);
        if (prev == null || !Number.isFinite(prev) || prev === 0 || Number.isNaN(mid)) return null;
        return ((mid - prev) / prev) * 100;
    }
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
    get showAvwap() { return this.micro().showAvwap; }
    set showAvwap(v: boolean) { this.micro().showAvwap = v; }
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
    get candleTimeframeLabel() {
        const sec = this.micro().barDurationSec;
        if (sec % 3600 === 0) return `${sec / 3600}h`;
        if (sec % 60 === 0) return `${sec / 60}m`;
        return `${sec}s`;
    }

    // Active instance assistant & chat accessors
    get assistantHistory() { return this.activeInstance().assistantHistory; }
    set assistantHistory(v: AssistantHistoryRecord[]) { this.activeInstance().assistantHistory = v; }
    get chatHistory() { return this.activeInstance().chatHistory; }
    set chatHistory(v: ChatMessage[]) { this.activeInstance().chatHistory = v; }
    get currentPosition(): 'None' | 'Long' | 'Short' { return this.activeInstance().currentPosition; }
    set currentPosition(v: 'None' | 'Long' | 'Short') {
        const pair = this.activeInstance(); const oldVal = pair.currentPosition;
        if (oldVal !== 'None' && v === 'None') { this.autoLogTrade(pair, oldVal); pair.entryPriceVal = ''; pair.stopLossVal = ''; }
        pair.currentPosition = v;
    }
    get entryPriceVal() { return this.activeInstance().entryPriceVal; }
    set entryPriceVal(v: string) { this.activeInstance().entryPriceVal = v; }
    get stopLossVal() { return this.activeInstance().stopLossVal; }
    set stopLossVal(v: string) { this.activeInstance().stopLossVal = v; }
    get assistantLoading() { return this.activeInstance().assistantLoading; }
    set assistantLoading(v: boolean) { this.activeInstance().assistantLoading = v; }
    get assistantError() { return this.activeInstance().assistantError; }
    set assistantError(v: string | null) { this.activeInstance().assistantError = v; }
    get assistantResponse() { return this.activeInstance().assistantResponse; }
    set assistantResponse(v: AssistantAnalysis | null) { this.activeInstance().assistantResponse = v; }
    get wizardResponse() { return this.activeInstance().wizardResponse; }
    set wizardResponse(v: WizardAnalysisResponse | null) { this.activeInstance().wizardResponse = v; }
    get analysisPhase() { return this.activeInstance().analysisPhase; }
    set analysisPhase(v: 'idle' | 'running' | 'complete') { this.activeInstance().analysisPhase = v; }
    get historyLatestClose() { return this.activeInstance().historyLatestClose; }
    set historyLatestClose(v: string) { this.activeInstance().historyLatestClose = v; }
    get isAssistantModalOpen() { return this.activeInstance().isAssistantModalOpen; }
    set isAssistantModalOpen(v: boolean) { this.activeInstance().isAssistantModalOpen = v; }
    get chatInputText() { return this.activeInstance().chatInputText; }
    set chatInputText(v: string) { this.activeInstance().chatInputText = v; }
    get isChatLoading() { return this.activeInstance().isChatLoading; }
    set isChatLoading(v: boolean) { this.activeInstance().isChatLoading = v; }
    get currentView() { return this.activeInstance().currentView; }
    set currentView(v: CurrentView) {
        const pair = this.activeInstance();
        pair.currentView = v;
        pair.modeViews[pair.currentLevel2Mode] = v;
    }

    // ─── Level 2 operational-mode navigation (UI-only) ────────────────
    get currentLevel2Mode() { return this.activeInstance().currentLevel2Mode; }
    set currentLevel2Mode(m: Level2Mode) { this.switchMode(m); }

    /** Switch the Level 2 paradigm and restore that mode's last active Level 3 view. */
    switchMode(mode: Level2Mode) {
        const pair = this.activeInstance();
        pair.currentLevel2Mode = mode;
        pair.currentView = pair.modeViews[mode];
    }

    get activeExecutionMode() { return this.activeInstance().activeExecutionMode; }

    /** operational_mode that the currently selected paradigm maps to (null = leave unchanged). */
    get pendingOperationalMode(): OperationalMode | null {
        return MODE_TO_OP[this.activeInstance().currentLevel2Mode];
    }

    /** Explicitly apply the selected paradigm's operational_mode to the backend. */
    async applyMode(): Promise<boolean> {
        const target = this.pendingOperationalMode;
        if (!target) return false;
        try {
            const res = await fetch(`/api/instances/${encodeURIComponent(this.activeTab)}/config`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ operational_mode: target }),
            });
            if (res.ok) { this.activeInstance().activeExecutionMode = target; return true; }
            return false;
        } catch (e) {
            console.error('Failed to apply operational mode:', e);
            return false;
        }
    }

    // Automation accessors
    get automationEnabled() { return this.activeInstance().automationEnabled; }
    set automationEnabled(v: boolean) { this.activeInstance().automationEnabled = v; }
    get automationIntervalValue() { return this.activeInstance().automationIntervalValue; }
    set automationIntervalValue(v: number) { this.activeInstance().automationIntervalValue = v; }
    get automationIntervalUnit() { return this.activeInstance().automationIntervalUnit; }
    set automationIntervalUnit(v: 'seconds' | 'minutes' | 'hours') { this.activeInstance().automationIntervalUnit = v; }
    get slowIntervalSecs() { return this.activeInstance().slowIntervalSecs; }
    set slowIntervalSecs(v: number) { this.activeInstance().slowIntervalSecs = v; }
    get normalIntervalSecs() { return this.activeInstance().normalIntervalSecs; }
    set normalIntervalSecs(v: number) { this.activeInstance().normalIntervalSecs = v; }
    get fastIntervalSecs() { return this.activeInstance().fastIntervalSecs; }
    set fastIntervalSecs(v: number) { this.activeInstance().fastIntervalSecs = v; }
    get nextEvaluationIn() { return this.activeInstance().nextEvaluationIn; }
    set nextEvaluationIn(v: string) { this.activeInstance().nextEvaluationIn = v; }

    // Confluence & S/R accessors
    get totalPointsScore() { return this.activeInstance().totalPointsScore; }
    set totalPointsScore(v: number) { this.activeInstance().totalPointsScore = v; }
    get allocatedCapitalPct() { return this.activeInstance().allocatedCapitalPct; }
    set allocatedCapitalPct(v: number) { this.activeInstance().allocatedCapitalPct = v; }
    get activeOppositeSignalsCount() { return this.activeInstance().activeOppositeSignalsCount; }
    set activeOppositeSignalsCount(v: number) { this.activeInstance().activeOppositeSignalsCount = v; }
    get markedSupportLevels() { return this.activeInstance().markedSupportLevels; }
    set markedSupportLevels(v: number[]) { this.activeInstance().markedSupportLevels = v; }
    get markedResistanceLevels() { return this.activeInstance().markedResistanceLevels; }
    set markedResistanceLevels(v: number[]) { this.activeInstance().markedResistanceLevels = v; }
    get srFlipEvents() { return this.activeInstance().srFlipEvents; }
    set srFlipEvents(v: string) { this.activeInstance().srFlipEvents = v; }

    // ─── Delegate Methods (with activeTab/sessionCapital references) ────

    async fetchCostEstimate() { await this.settings.fetchCostEstimate(this.activeTab); }
    async fetchRiskProfile() {
        this.riskProfileLoading = true;
        this.riskProfileError = null;
        try {
            const res = await fetch(`/api/risk-profile?pair_key=${encodeURIComponent(this.activeTab)}&_=${Date.now()}`);
            if (!res.ok) throw new Error(`Risk profile fetch failed: ${res.status}`);
            this.riskProfile = await res.json();
        } catch (e) {
            this.riskProfileError = e instanceof Error ? e.message : String(e);
            this.riskProfile = null;
        } finally {
            this.riskProfileLoading = false;
        }
    }
    async fetchPaperStatus() { await this.paper.fetchPaperStatus(this.activeTab); }
    async fetchActiveTrades() { /* no-op; MonitoringPanel fetches directly */ }
    async openPaperPosition(direction: 'LONG' | 'SHORT') { await this.paper.openPaperPosition(this.activeTab, direction); }
    async closePaperPosition() { await this.paper.closePaperPosition(this.activeTab); }
    async openPositionPct(direction: 'LONG' | 'SHORT', pct: number) { return await this.paper.openPositionPct(this.activeTab, direction, pct); }
    async closePositionPct(pct: number) { return await this.paper.closePositionPct(this.activeTab, pct); }
    async setTpTargets(targets: { pct: number; price: number }[]) { return await this.paper.setTpTargets(this.activeTab, targets); }
    async setSlLevels(stops: { pct: number; price: number }[]) { return await this.paper.setSlLevels(this.activeTab, stops); }
    async resetPaperAccount() { await this.paper.resetPaperAccount(this.activeTab); }
    async savePaperConfig(initialUSD: number, allocationPct: number, autoExecute: boolean) { await this.paper.savePaperConfig(this.activeTab, initialUSD, allocationPct, autoExecute); }
    async fetchPaperHistory(symbol?: string) { await this.paper.fetchPaperHistory(this.activeTab, symbol); }
    async fetchOpenOrders() { await this.paper.fetchOpenOrders(this.activeTab); }
    async placeOrder(order: import('./types').PlaceOrderPayload) { return await this.paper.placeOrder(this.activeTab, order); }
    async cancelOrder(orderId: number) { return await this.paper.cancelOrder(this.activeTab, orderId); }
    async fetchSlotStates() { await this.paper.fetchSlotStates(this.activeTab); }
    async fetchEquityHistory() { await this.paper.fetchEquityHistory(this.activeTab); }
    async openSlot(direction: 'LONG' | 'SHORT') { return await this.paper.openSlot(this.activeTab, direction); }
    async closeSlot() { return await this.paper.closeSlot(this.activeTab); }

    async fetchDashboardStats() { await this.analytics.fetchDashboardStats(this.sessionCapital); }
}

// Module-level singleton for backward compatibility
const store = new AppStore();

export function useAppStore(): AppStore {
    return store;
}

export function createAppStore(): AppStore {
    return new AppStore();
}
