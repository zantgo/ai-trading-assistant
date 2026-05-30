// Global reactive state using Svelte 5 runes

export interface TrendAnalysis {
    classification: string;
    structural_reasoning: string;
}

export interface IndicatorAlignment {
    classification: string;
    observation: string;
}

export interface PositionRecommendation {
    action: string;
    rationale: string;
}

export interface AssistantAnalysis {
    trend_analysis: TrendAnalysis;
    indicator_alignment: IndicatorAlignment;
    position_recommendation: PositionRecommendation;
}

export interface AssistantHistoryRecord {
    id: number;
    created_at: string;
    position: string;
    entry_price?: string;
    trend_classification: string;
    indicator_alignment: string;
    indicator_synthesis_summary?: string;
    recommended_action: string;
    recommendation_rationale: string;
    price_at_analysis: string;
    support_levels?: string;
    resistance_levels?: string;
    symbol: string;
}

export interface ChatMessage {
    role: string;
    content: string;
}

export interface IndividualIndicatorResult {
    indicator_name: string;
    signal: 'BULLISH' | 'BEARISH' | 'SIDEWAYS' | 'UNAVAILABLE';
    reason: string;
}

export interface SupportResistance {
    detected_support_levels: string[];
    detected_resistance_levels: string[];
    structural_analysis: string;
}

export interface IndicatorSynthesis {
    summary_count: string;
    evaluation: string;
}

export interface MasterOrchestratorResult {
    general_trend: 'UPWARD' | 'DOWNWARD' | 'SIDEWAYS';
    support_and_resistance: SupportResistance;
    indicator_synthesis: IndicatorSynthesis;
    position_recommendation: { action: string; rationale: string; };
}

export interface MultiAgentAnalysis {
    phase_one: IndividualIndicatorResult[];
    phase_two: MasterOrchestratorResult;
}

export interface AgentProgress {
    name: string;
    status: 'pending' | 'running' | 'complete' | 'failed';
}

export interface PairState {
    symbol: string;
    exchange: string;
    isConnected: boolean;
    priceText: string;
    vwapText: string;
    avgVolText: string;
    emaFastText: string;
    emaMediumText: string;
    emaSlowText: string;
    emaLongText: string;
    adxText: string;
    adxPlusText: string;
    adxMinusText: string;
    atrText: string;
    rsiText: string;
    macdLineText: string;
    macdSigText: string;
    macdHistText: string;
    sqzValText: string;
    sqzStatusText: string;
    isSqueezeOn: boolean;
    volText: string;
    lastMacdHist: number;
    lastSqzMom: number;
    latestSnapshot: Record<string, unknown> | null;
    assistantHistory: AssistantHistoryRecord[];
    chatHistory: ChatMessage[];
    historyPrices: number[];
    currentPosition: 'None' | 'Long' | 'Short';
    entryPriceVal: string;
    assistantLoading: boolean;
    assistantError: string | null;
    assistantResponse: AssistantAnalysis | null;
    multiAgentResponse: MultiAgentAnalysis | null;
    analysisPhase: 'idle' | 'phase1' | 'phase2' | 'complete';
    individualResults: IndividualIndicatorResult[];
    agentProgress: AgentProgress[];
    historyLatestClose: string;
    isAssistantModalOpen: boolean;
    chatInputText: string;
    isChatLoading: boolean;

    // Per-pair configuration
    barDurationSec: number;
    emaFastVal: number;
    emaMediumVal: number;
    emaSlowVal: number;
    emaLongVal: number;
    rsiPeriodVal: number;
    macdFastVal: number;
    macdSlowVal: number;
    macdSignalVal: number;
    adxPeriodVal: number;
    atrPeriodVal: number;
    squeezePeriodVal: number;

    // Per-pair visibility
    showEmas: boolean;
    showBb: boolean;
    showVwap: boolean;
    showVolume: boolean;
    showAdx: boolean;
    showAtr: boolean;
    showRsi: boolean;
    showMacd: boolean;
    showSqueeze: boolean;
}

function createPairState(symbol: string, exchange: string): PairState {
    return {
        symbol,
        exchange,
        isConnected: false,
        priceText: '--',
        vwapText: '--',
        avgVolText: '--',
        emaFastText: '--',
        emaMediumText: '--',
        emaSlowText: '--',
        emaLongText: '--',
        adxText: '--',
        adxPlusText: '--',
        adxMinusText: '--',
        atrText: '--',
        rsiText: '--',
        macdLineText: '--',
        macdSigText: '--',
        macdHistText: '--',
        sqzValText: '--',
        sqzStatusText: '--',
        isSqueezeOn: false,
        volText: '--',
        lastMacdHist: 0,
        lastSqzMom: 0,
        latestSnapshot: null,
        assistantHistory: [],
        chatHistory: [],
        historyPrices: [],
        currentPosition: 'None',
        entryPriceVal: '',
        assistantLoading: false,
        assistantError: null,
        assistantResponse: null,
        multiAgentResponse: null,
        analysisPhase: 'idle',
        individualResults: [],
        agentProgress: [],
        historyLatestClose: '0',
        isAssistantModalOpen: false,
        chatInputText: '',
        isChatLoading: false,

        barDurationSec: 60,
        emaFastVal: 10,
        emaMediumVal: 50,
        emaSlowVal: 100,
        emaLongVal: 200,
        rsiPeriodVal: 14,
        macdFastVal: 12,
        macdSlowVal: 26,
        macdSignalVal: 9,
        adxPeriodVal: 14,
        atrPeriodVal: 14,
        squeezePeriodVal: 20,

        showEmas: true,
        showBb: true,
        showVwap: true,
        showVolume: true,
        showAdx: true,
        showAtr: true,
        showRsi: true,
        showMacd: true,
        showSqueeze: true,
    };
}

// --- Per-pair state map ---
let pairsMap = $state<Record<string, PairState>>({});
let activeTab = $state<string>('Hyperliquid-BTC');

// --- Global configuration ---
let apiKeyConfigured = $state(true);
let rulesContent = $state('');
let showSettingsPanel = $state(false);

// Labels (global for display)
let emaFastLabel = $state('EMA-10');
let emaMediumLabel = $state('EMA-50');
let emaSlowLabel = $state('EMA-100');
let emaLongLabel = $state('EMA-200');
let rsiLabel = $state('RSI (14)');
let adxLabel = $state('ADX (14)');
let atrLabel = $state('ATR (14)');
let macdLabel = $state('MACD (12,26,9)');

// --- Helper to get/set active pair ---
function activePair(): PairState {
    if (!pairsMap[activeTab]) {
        const parts = activeTab.split('-');
        pairsMap[activeTab] = createPairState(parts[1] || 'BTC', parts[0] || 'Hyperliquid');
    }
    return pairsMap[activeTab];
}

export function initPair(symbol: string, exchange: string = 'Hyperliquid') {
    const key = `${exchange}-${symbol}`;
    if (!pairsMap[key]) {
        pairsMap[key] = createPairState(symbol, exchange);
    }
}

export function removePair(key: string) {
    delete pairsMap[key];
}

export function switchTab(key: string) {
    activeTab = key;
}

export function getState() {
    const app = {
        initPair(symbol: string, exchange: string = 'Hyperliquid') { initPair(symbol, exchange); },
        removePair(key: string) { removePair(key); },
        get pairsMap() { return pairsMap; },
        get activeTab() { return activeTab; },
        set activeTab(v: string) { activeTab = v; },

        // Global
        get apiKeyConfigured() { return apiKeyConfigured; },
        set apiKeyConfigured(v: boolean) { apiKeyConfigured = v; },
        get rulesContent() { return rulesContent; },
        set rulesContent(v: string) { rulesContent = v; },
        get showSettingsPanel() { return showSettingsPanel; },
        set showSettingsPanel(v: boolean) { showSettingsPanel = v; },

        // Visibility — proxied per-pair
        get showEmas() { return activePair().showEmas; }, set showEmas(v: boolean) { activePair().showEmas = v; },
        get showBb() { return activePair().showBb; }, set showBb(v: boolean) { activePair().showBb = v; },
        get showVwap() { return activePair().showVwap; }, set showVwap(v: boolean) { activePair().showVwap = v; },
        get showVolume() { return activePair().showVolume; }, set showVolume(v: boolean) { activePair().showVolume = v; },
        get showAdx() { return activePair().showAdx; }, set showAdx(v: boolean) { activePair().showAdx = v; },
        get showAtr() { return activePair().showAtr; }, set showAtr(v: boolean) { activePair().showAtr = v; },
        get showRsi() { return activePair().showRsi; }, set showRsi(v: boolean) { activePair().showRsi = v; },
        get showMacd() { return activePair().showMacd; }, set showMacd(v: boolean) { activePair().showMacd = v; },
        get showSqueeze() { return activePair().showSqueeze; }, set showSqueeze(v: boolean) { activePair().showSqueeze = v; },

        // Indicator config — proxied per-pair
        get barDurationSec() { return activePair().barDurationSec; }, set barDurationSec(v: number) { activePair().barDurationSec = v; },
        get emaFastVal() { return activePair().emaFastVal; }, set emaFastVal(v: number) { activePair().emaFastVal = v; },
        get emaMediumVal() { return activePair().emaMediumVal; }, set emaMediumVal(v: number) { activePair().emaMediumVal = v; },
        get emaSlowVal() { return activePair().emaSlowVal; }, set emaSlowVal(v: number) { activePair().emaSlowVal = v; },
        get emaLongVal() { return activePair().emaLongVal; }, set emaLongVal(v: number) { activePair().emaLongVal = v; },
        get rsiPeriodVal() { return activePair().rsiPeriodVal; }, set rsiPeriodVal(v: number) { activePair().rsiPeriodVal = v; },
        get macdFastVal() { return activePair().macdFastVal; }, set macdFastVal(v: number) { activePair().macdFastVal = v; },
        get macdSlowVal() { return activePair().macdSlowVal; }, set macdSlowVal(v: number) { activePair().macdSlowVal = v; },
        get macdSignalVal() { return activePair().macdSignalVal; }, set macdSignalVal(v: number) { activePair().macdSignalVal = v; },
        get adxPeriodVal() { return activePair().adxPeriodVal; }, set adxPeriodVal(v: number) { activePair().adxPeriodVal = v; },
        get atrPeriodVal() { return activePair().atrPeriodVal; }, set atrPeriodVal(v: number) { activePair().atrPeriodVal = v; },
        get squeezePeriodVal() { return activePair().squeezePeriodVal; }, set squeezePeriodVal(v: number) { activePair().squeezePeriodVal = v; },

        // Dynamic timeframe label
        get candleTimeframeLabel() {
            const sec = activePair().barDurationSec;
            if (sec % 3600 === 0) return `${sec / 3600}h`;
            if (sec % 60 === 0) return `${sec / 60}m`;
            return `${sec}s`;
        },

        // Labels
        get emaFastLabel() { return emaFastLabel; }, set emaFastLabel(v: string) { emaFastLabel = v; },
        get emaMediumLabel() { return emaMediumLabel; }, set emaMediumLabel(v: string) { emaMediumLabel = v; },
        get emaSlowLabel() { return emaSlowLabel; }, set emaSlowLabel(v: string) { emaSlowLabel = v; },
        get emaLongLabel() { return emaLongLabel; }, set emaLongLabel(v: string) { emaLongLabel = v; },
        get rsiLabel() { return rsiLabel; }, set rsiLabel(v: string) { rsiLabel = v; },
        get adxLabel() { return adxLabel; }, set adxLabel(v: string) { adxLabel = v; },
        get atrLabel() { return atrLabel; }, set atrLabel(v: string) { atrLabel = v; },
        get macdLabel() { return macdLabel; }, set macdLabel(v: string) { macdLabel = v; },

        // Proxied per-pair fields
        get activeSymbol() { return activePair().symbol; },
        get activeExchange() { return activePair().exchange; },
        get isConnected() { return activePair().isConnected; },
        set isConnected(v: boolean) { activePair().isConnected = v; },
        get priceText() { return activePair().priceText; },
        set priceText(v: string) { activePair().priceText = v; },
        get vwapText() { return activePair().vwapText; },
        set vwapText(v: string) { activePair().vwapText = v; },
        get avgVolText() { return activePair().avgVolText; },
        set avgVolText(v: string) { activePair().avgVolText = v; },
        get emaFastText() { return activePair().emaFastText; },
        set emaFastText(v: string) { activePair().emaFastText = v; },
        get emaMediumText() { return activePair().emaMediumText; },
        set emaMediumText(v: string) { activePair().emaMediumText = v; },
        get emaSlowText() { return activePair().emaSlowText; },
        set emaSlowText(v: string) { activePair().emaSlowText = v; },
        get emaLongText() { return activePair().emaLongText; },
        set emaLongText(v: string) { activePair().emaLongText = v; },
        get adxText() { return activePair().adxText; },
        set adxText(v: string) { activePair().adxText = v; },
        get adxPlusText() { return activePair().adxPlusText; },
        set adxPlusText(v: string) { activePair().adxPlusText = v; },
        get adxMinusText() { return activePair().adxMinusText; },
        set adxMinusText(v: string) { activePair().adxMinusText = v; },
        get atrText() { return activePair().atrText; },
        set atrText(v: string) { activePair().atrText = v; },
        get rsiText() { return activePair().rsiText; },
        set rsiText(v: string) { activePair().rsiText = v; },
        get macdLineText() { return activePair().macdLineText; },
        set macdLineText(v: string) { activePair().macdLineText = v; },
        get macdSigText() { return activePair().macdSigText; },
        set macdSigText(v: string) { activePair().macdSigText = v; },
        get macdHistText() { return activePair().macdHistText; },
        set macdHistText(v: string) { activePair().macdHistText = v; },
        get sqzValText() { return activePair().sqzValText; },
        set sqzValText(v: string) { activePair().sqzValText = v; },
        get sqzStatusText() { return activePair().sqzStatusText; },
        set sqzStatusText(v: string) { activePair().sqzStatusText = v; },
        get isSqueezeOn() { return activePair().isSqueezeOn; },
        set isSqueezeOn(v: boolean) { activePair().isSqueezeOn = v; },
        get volText() { return activePair().volText; },
        set volText(v: string) { activePair().volText = v; },
        get lastMacdHist() { return activePair().lastMacdHist; },
        set lastMacdHist(v: number) { activePair().lastMacdHist = v; },
        get lastSqzMom() { return activePair().lastSqzMom; },
        set lastSqzMom(v: number) { activePair().lastSqzMom = v; },
        get latestSnapshot() { return activePair().latestSnapshot; },
        set latestSnapshot(v: Record<string, unknown> | null) { activePair().latestSnapshot = v; },
        get assistantHistory() { return activePair().assistantHistory; },
        set assistantHistory(v: AssistantHistoryRecord[]) { activePair().assistantHistory = v; },
        get chatHistory() { return activePair().chatHistory; },
        set chatHistory(v: ChatMessage[]) { activePair().chatHistory = v; },
        get historyPrices() { return activePair().historyPrices; },
        set historyPrices(v: number[]) { activePair().historyPrices = v; },
        get currentPosition() { return activePair().currentPosition; },
        set currentPosition(v: 'None' | 'Long' | 'Short') { activePair().currentPosition = v; },
        get entryPriceVal() { return activePair().entryPriceVal; },
        set entryPriceVal(v: string) { activePair().entryPriceVal = v; },
        get assistantLoading() { return activePair().assistantLoading; },
        set assistantLoading(v: boolean) { activePair().assistantLoading = v; },
        get assistantError() { return activePair().assistantError; },
        set assistantError(v: string | null) { activePair().assistantError = v; },
        get assistantResponse() { return activePair().assistantResponse; },
        set assistantResponse(v: AssistantAnalysis | null) { activePair().assistantResponse = v; },
        get multiAgentResponse() { return activePair().multiAgentResponse; },
        set multiAgentResponse(v: MultiAgentAnalysis | null) { activePair().multiAgentResponse = v; },
        get analysisPhase() { return activePair().analysisPhase; },
        set analysisPhase(v: 'idle' | 'phase1' | 'phase2' | 'complete') { activePair().analysisPhase = v; },
        get individualResults() { return activePair().individualResults; },
        set individualResults(v: IndividualIndicatorResult[]) { activePair().individualResults = v; },
        get agentProgress() { return activePair().agentProgress; },
        set agentProgress(v: AgentProgress[]) { activePair().agentProgress = v; },
        get historyLatestClose() { return activePair().historyLatestClose; },
        set historyLatestClose(v: string) { activePair().historyLatestClose = v; },
        get isAssistantModalOpen() { return activePair().isAssistantModalOpen; },
        set isAssistantModalOpen(v: boolean) { activePair().isAssistantModalOpen = v; },
        get chatInputText() { return activePair().chatInputText; },
        set chatInputText(v: string) { activePair().chatInputText = v; },
        get isChatLoading() { return activePair().isChatLoading; },
        set isChatLoading(v: boolean) { activePair().isChatLoading = v; },
    };

    return app;
}
