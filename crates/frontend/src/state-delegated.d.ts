import type {
    DecisionProfile, DecisionScore, RiskProfile, RiskCalculation,
    FeeTableRow, CommissionProjection, ExchangeAccount,
    DashboardStats, TradeLedgerRecord, TradeJournalRecord,
    ScaleInPortion, TakeProfitTarget, UserTrade,
    SystemHeartbeat, DecisionMemoryRow, CompletedTradesRow,
    OpenOrder, SlotState, PositionSlot, EquitySnapshot,
} from './types';

declare module './state.svelte' {
    interface AppStore {
        sessionActive: boolean; sessionMode: string; sessionCurrency: string;
        sessionExchange: string; sessionCapital: number; sessionInstanceCount: number;
        sessionMaxInstances: number; sessionLoading: boolean; sessionChecked: boolean;
        sessionError: string | null;

        activeDecisionProfileId: number; decisionProfiles: DecisionProfile[];
        calculatedDecisionScore: DecisionScore | null; decisionLoading: boolean;
        activeRiskProfileId: number; riskProfiles: RiskProfile[];
        riskDirection: 'LONG' | 'SHORT'; riskEntryPrice: string; riskStopLoss: string;
        riskTakeProfit: string; riskCalculation: RiskCalculation | null;
        riskCalculating: boolean; useDynamicAtr: boolean; atrValue: number;

        commissionDirection: 'LONG' | 'SHORT'; commissionEntry1: string;
        commissionEntry2: string; commissionSL1: string; commissionSL2: string;
        commissionTP1: string; commissionTP2: string; commissionCapitalSplit: number;
        commissionOrderType: 'maker' | 'taker'; commissionProjection: CommissionProjection | null;
        commissionLoading: boolean; feeTable: FeeTableRow[]; feeTableLoading: boolean;

        exchangeAccounts: ExchangeAccount[]; exchangeActiveCount: number;
        exchangeMaxAccounts: number;
        exchangeFormDraft: { exchange: string; account_name: string; api_key: string; api_secret: string; passphrase: string; referred_uid: string; is_active: boolean };

        paperCashBalance: number; paperInitialUSD: number; paperAllocationPct: number;
        paperAutoExecute: boolean; activePaperPosition: Record<string, unknown> | null;
        paperUnrealizedPnl: number; paperUnrealizedRoi: number;
        paperTotalAccountValue: number; paperMarginUsed: number; paperMaxTrades: number;
        paperActiveTrades: number; paperAvailableTrades: number;
        paperHistory: Record<string, unknown>[]; paperLoading: boolean;
        paperScaleInPortions: ScaleInPortion[]; paperTakeProfitTargets: TakeProfitTarget[];
        paperAvgEntryPrice: number; paperInvalidationLevel: number;
        paperFilledPortions: number; paperMaxRiskPct: number; paperLeverage: number;
        paperAutoExecuteIntervals: number; paperLookbackTrades: number;
        paperPositionPct: number; paperFreeBalancePct: number; paperDirection: 'LONG' | 'SHORT' | '';
        openOrders: OpenOrder[];
        activeSlots: SlotState[]; positionSlots: PositionSlot[];
        equitySnapshots: EquitySnapshot[];
        paperInitialAllocatedMargin: number;
        paperRealizedPnlAccumulator: number;
        paperBreakEvenTrailEnabled: boolean;

        apiKeyConfigured: boolean; rulesContent: string;
        globalCandlesConfig: { duration_seconds: number; analysis_limit: number };
        globalIndicatorsConfig: Record<string, number>;
        indicatorRegistry: import('./types').IndicatorMeta[];
        emaFastLabel: string; emaMediumLabel: string; emaSlowLabel: string; emaLongLabel: string;
        rsiLabel: string; adxLabel: string; atrLabel: string; macdLabel: string;

        costPriceInput: number; costPriceOutput: number; costIntervalSecs: number;
        costRunsPerDay: number; costTokensPerRunInput: number; costTokensPerRunOutput: number;
        costDailyProjected: number; costWeeklyProjected: number; costMonthlyProjected: number;
        costActualInputTokens: number; costActualOutputTokens: number; costActualTotal: number;
        costLoading: boolean;

        dashboardStats: DashboardStats | null; dashboardActiveFilter: string;
        dashboardPeriod: string; dashboardOrigin: string;
        tradeLedgerRecords: TradeLedgerRecord[]; tradeJournalRecords: TradeJournalRecord[];
        journalLookbackDepth: number; systemHeartbeat: SystemHeartbeat | null;
        recentDecisions: DecisionMemoryRow[]; completedTrades: CompletedTradesRow[];
        userTrades: UserTrade[];

        fetchSessionStatus(): Promise<void>;
        initSession(mode: string, currency: string, exchange: string, capital: number): Promise<any>;
        quitSession(): Promise<any>;
        fetchDecisionProfiles(): Promise<void>;
        createDecisionProfile(name: string, longT: number, shortT: number): Promise<void>;
        deleteDecisionProfile(id: number): Promise<void>;
        updateDecisionProfileThresholds(id: number, longT: number, shortT: number): Promise<void>;
        addProfileIndicator(profileId: number, name: string, weight: number, overrideStatus: string): Promise<void>;
        updateProfileIndicator(profileId: number, indicatorId: number, weight: number, overrideStatus: string): Promise<void>;
        deleteProfileIndicator(profileId: number, indicatorId: number): Promise<void>;
        fetchRiskProfiles(): Promise<void>;
        createRiskProfile(name: string, capital: number, riskPct: number, leverage: number): Promise<void>;
        deleteRiskProfile(id: number): Promise<void>;
        calculateRisk(): Promise<void>;
        fetchFeeTable(): Promise<void>;
        calculateCommissionProjection(): Promise<void>;
        fetchExchangeKeys(): Promise<void>;
        addExchangeKey(): Promise<void>;
        deleteExchangeKey(id: number): Promise<void>;
        fetchTradeLedger(): Promise<void>;
        fetchTradeJournal(limit?: number): Promise<void>;
        updateJournalNotes(id: number, notes: string, score: number): Promise<void>;
        fetchSystemStatus(): Promise<void>;
        fetchObservabilityBuffers(symbol: string): Promise<void>;
        fetchTrades(): Promise<void>;
        exportJournalCSV(): void;
        exportJournalJSON(): void;
    }
}
