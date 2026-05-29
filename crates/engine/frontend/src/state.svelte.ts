// Global reactive state using Svelte 5 runes

let isConnected = $state(false);
let priceText = $state('--');
let emaFastText = $state('--');
let emaMediumText = $state('--');
let emaSlowText = $state('--');
let emaLongText = $state('--');
let adxText = $state('--');
let adxPlusText = $state('--');
let adxMinusText = $state('--');
let atrText = $state('--');
let rsiText = $state('--');
let macdLineText = $state('--');
let macdSigText = $state('--');
let macdHistText = $state('--');
let sqzValText = $state('--');
let sqzStatusText = $state('Calculating');
let isSqueezeOn = $state(false);
let volText = $state('--');
let vwapText = $state('--');
let avgVolText = $state('--');

let activeSymbol = $state('ETH');
let candleTimeframeLabel = $state('5s');

let emaFastLabel = $state('EMA Fast');
let emaMediumLabel = $state('EMA Med');
let emaSlowLabel = $state('EMA Slow');
let emaLongLabel = $state('EMA Long');
let rsiLabel = $state('RSI (14)');
let adxLabel = $state('ADX (14)');
let atrLabel = $state('ATR (14)');
let macdLabel = $state('MACD (12, 26, 9)');

let showEmas = $state(true);
let showBb = $state(true);
let showVwap = $state(true);
let showVolume = $state(true);
let showAdx = $state(true);
let showAtr = $state(true);
let showRsi = $state(true);
let showMacd = $state(true);
let showSqueeze = $state(true);

let barDurationSec = $state(5);
let lastMacdHist = $state(0);
let lastSqzMom = $state(0);

let latestSnapshot: Record<string, unknown> | null = $state(null);

// Settings numerical values loaded from config.toml
let emaFastVal = $state(10);
let emaMediumVal = $state(50);
let emaSlowVal = $state(100);
let emaLongVal = $state(200);
let rsiPeriodVal = $state(14);
let macdFastVal = $state(12);
let macdSlowVal = $state(26);
let macdSignalVal = $state(9);
let adxPeriodVal = $state(14);
let atrPeriodVal = $state(14);
let squeezePeriodVal = $state(20);

// AI Assistant state
let currentPosition = $state<'None' | 'Long' | 'Short'>('None');
let entryPriceVal = $state('');
let assistantLoading = $state(false);
let assistantError = $state<string | null>(null);
let assistantResponse = $state<AssistantAnalysis | null>(null);
let multiAgentResponse = $state<MultiAgentAnalysis | null>(null);
let analysisPhase = $state<'idle' | 'phase1' | 'phase2' | 'complete'>('idle');
let individualResults = $state<IndividualIndicatorResult[]>([]);
let agentProgress = $state<AgentProgress[]>([]);
let historyPrices = $state<number[]>([]);
let assistantHistory = $state<AssistantHistoryRecord[]>([]);
let historyLatestClose = $state('0');

// Modal & Chat state
let isAssistantModalOpen = $state(false);
let chatInputText = $state('');
let chatHistory = $state<ChatMessage[]>([]);
let isChatLoading = $state(false);

let showSettingsPanel = $state(false);

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
    position_recommendation: {
        action: string;
        rationale: string;
    };
}

export interface MultiAgentAnalysis {
    phase_one: IndividualIndicatorResult[];
    phase_two: MasterOrchestratorResult;
}

export interface AgentProgress {
    name: string;
    status: 'pending' | 'running' | 'complete' | 'failed';
}

export function getState() {
    return {
        get isConnected() { return isConnected },
        set isConnected(v: boolean) { isConnected = v },
        get priceText() { return priceText },
        set priceText(v: string) { priceText = v },
        get emaFastText() { return emaFastText },
        set emaFastText(v: string) { emaFastText = v },
        get emaMediumText() { return emaMediumText },
        set emaMediumText(v: string) { emaMediumText = v },
        get emaSlowText() { return emaSlowText },
        set emaSlowText(v: string) { emaSlowText = v },
        get emaLongText() { return emaLongText },
        set emaLongText(v: string) { emaLongText = v },
        get adxText() { return adxText },
        set adxText(v: string) { adxText = v },
        get adxPlusText() { return adxPlusText },
        set adxPlusText(v: string) { adxPlusText = v },
        get adxMinusText() { return adxMinusText },
        set adxMinusText(v: string) { adxMinusText = v },
        get atrText() { return atrText },
        set atrText(v: string) { atrText = v },
        get rsiText() { return rsiText },
        set rsiText(v: string) { rsiText = v },
        get macdLineText() { return macdLineText },
        set macdLineText(v: string) { macdLineText = v },
        get macdSigText() { return macdSigText },
        set macdSigText(v: string) { macdSigText = v },
        get macdHistText() { return macdHistText },
        set macdHistText(v: string) { macdHistText = v },
        get sqzValText() { return sqzValText },
        set sqzValText(v: string) { sqzValText = v },
        get sqzStatusText() { return sqzStatusText },
        set sqzStatusText(v: string) { sqzStatusText = v },
        get isSqueezeOn() { return isSqueezeOn },
        set isSqueezeOn(v: boolean) { isSqueezeOn = v },
        get volText() { return volText },
        set volText(v: string) { volText = v },
        get vwapText() { return vwapText },
        set vwapText(v: string) { vwapText = v },
        get avgVolText() { return avgVolText },
        set avgVolText(v: string) { avgVolText = v },
        get activeSymbol() { return activeSymbol },
        set activeSymbol(v: string) { activeSymbol = v },
        get candleTimeframeLabel() { return candleTimeframeLabel },
        set candleTimeframeLabel(v: string) { candleTimeframeLabel = v },
        get emaFastLabel() { return emaFastLabel },
        set emaFastLabel(v: string) { emaFastLabel = v },
        get emaMediumLabel() { return emaMediumLabel },
        set emaMediumLabel(v: string) { emaMediumLabel = v },
        get emaSlowLabel() { return emaSlowLabel },
        set emaSlowLabel(v: string) { emaSlowLabel = v },
        get emaLongLabel() { return emaLongLabel },
        set emaLongLabel(v: string) { emaLongLabel = v },
        get rsiLabel() { return rsiLabel },
        set rsiLabel(v: string) { rsiLabel = v },
        get adxLabel() { return adxLabel },
        set adxLabel(v: string) { adxLabel = v },
        get atrLabel() { return atrLabel },
        set atrLabel(v: string) { atrLabel = v },
        get macdLabel() { return macdLabel },
        set macdLabel(v: string) { macdLabel = v },
        get showEmas() { return showEmas },
        set showEmas(v: boolean) { showEmas = v },
        get showBb() { return showBb },
        set showBb(v: boolean) { showBb = v },
        get showVwap() { return showVwap },
        set showVwap(v: boolean) { showVwap = v },
        get showVolume() { return showVolume },
        set showVolume(v: boolean) { showVolume = v },
        get showAdx() { return showAdx },
        set showAdx(v: boolean) { showAdx = v },
        get showAtr() { return showAtr },
        set showAtr(v: boolean) { showAtr = v },
        get showRsi() { return showRsi },
        set showRsi(v: boolean) { showRsi = v },
        get showMacd() { return showMacd },
        set showMacd(v: boolean) { showMacd = v },
        get showSqueeze() { return showSqueeze },
        set showSqueeze(v: boolean) { showSqueeze = v },
        get barDurationSec() { return barDurationSec },
        set barDurationSec(v: number) { barDurationSec = v },
        get lastMacdHist() { return lastMacdHist },
        set lastMacdHist(v: number) { lastMacdHist = v },
        get lastSqzMom() { return lastSqzMom },
        set lastSqzMom(v: number) { lastSqzMom = v },
        get latestSnapshot() { return latestSnapshot },
        set latestSnapshot(v: Record<string, unknown> | null) { latestSnapshot = v },
        get emaFastVal() { return emaFastVal },
        set emaFastVal(v: number) { emaFastVal = v },
        get emaMediumVal() { return emaMediumVal },
        set emaMediumVal(v: number) { emaMediumVal = v },
        get emaSlowVal() { return emaSlowVal },
        set emaSlowVal(v: number) { emaSlowVal = v },
        get emaLongVal() { return emaLongVal },
        set emaLongVal(v: number) { emaLongVal = v },
        get rsiPeriodVal() { return rsiPeriodVal },
        set rsiPeriodVal(v: number) { rsiPeriodVal = v },
        get macdFastVal() { return macdFastVal },
        set macdFastVal(v: number) { macdFastVal = v },
        get macdSlowVal() { return macdSlowVal },
        set macdSlowVal(v: number) { macdSlowVal = v },
        get macdSignalVal() { return macdSignalVal },
        set macdSignalVal(v: number) { macdSignalVal = v },
        get adxPeriodVal() { return adxPeriodVal },
        set adxPeriodVal(v: number) { adxPeriodVal = v },
        get atrPeriodVal() { return atrPeriodVal },
        set atrPeriodVal(v: number) { atrPeriodVal = v },
        get squeezePeriodVal() { return squeezePeriodVal },
        set squeezePeriodVal(v: number) { squeezePeriodVal = v },
        get currentPosition() { return currentPosition },
        set currentPosition(v: 'None' | 'Long' | 'Short') { currentPosition = v },
        get entryPriceVal() { return entryPriceVal },
        set entryPriceVal(v: string) { entryPriceVal = v },
        get assistantLoading() { return assistantLoading },
        set assistantLoading(v: boolean) { assistantLoading = v },
        get assistantError() { return assistantError },
        set assistantError(v: string | null) { assistantError = v },
        get assistantResponse() { return assistantResponse },
        set assistantResponse(v: AssistantAnalysis | null) { assistantResponse = v },
        get multiAgentResponse() { return multiAgentResponse },
        set multiAgentResponse(v: MultiAgentAnalysis | null) { multiAgentResponse = v },
        get analysisPhase() { return analysisPhase },
        set analysisPhase(v: 'idle' | 'phase1' | 'phase2' | 'complete') { analysisPhase = v },
        get individualResults() { return individualResults },
        set individualResults(v: IndividualIndicatorResult[]) { individualResults = v },
        get agentProgress() { return agentProgress },
        set agentProgress(v: AgentProgress[]) { agentProgress = v },
        get historyPrices() { return historyPrices },
        set historyPrices(v: number[]) { historyPrices = v },
        get assistantHistory() { return assistantHistory },
        set assistantHistory(v: AssistantHistoryRecord[]) { assistantHistory = v },
        get historyLatestClose() { return historyLatestClose },
        set historyLatestClose(v: string) { historyLatestClose = v },
        get showSettingsPanel() { return showSettingsPanel },
        set showSettingsPanel(v: boolean) { showSettingsPanel = v },
        get isAssistantModalOpen() { return isAssistantModalOpen },
        set isAssistantModalOpen(v: boolean) { isAssistantModalOpen = v },
        get chatInputText() { return chatInputText },
        set chatInputText(v: string) { chatInputText = v },
        get chatHistory() { return chatHistory },
        set chatHistory(v: ChatMessage[]) { chatHistory = v },
        get isChatLoading() { return isChatLoading },
        set isChatLoading(v: boolean) { isChatLoading = v },
    };
}
