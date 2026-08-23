export interface StrategyAnalyticsRow {
    setup_type: string;
    /** Significance bar: an edge is significant when p_value and p_mc are both below this. */
    alpha: number;
    total_trades: number;
    win_count: number;
    loss_count: number;
    win_rate: number;
    gross_profit: number;
    gross_loss: number;
    profit_factor: number | null;
    average_win: number;
    average_loss: number;
    avg_win_loss_ratio: number;
    expectancy: number;
    slippage_overhead: number;
    t_statistic: number;
    p_value: number;
    p_mc: number;
    monte_carlo_runs: number;
    is_significant: boolean;
    classification: 'StrongEdge' | 'ModerateEdge' | 'WeakMarginalEdge' | 'NoEdgeNegative' | 'InsufficientData';
}

export interface RiskAnalyticsRow {
    maximum_drawdown_pct: number;
    max_drawdown_duration_days: number;
    average_drawdown_pct: number;
    drawdown_count: number;
    sharpe_ratio: number | null;
    sortino_ratio: number | null;
    ulcer_index: number;
    calmar_ratio: number | null;
    daily_volatility: number;
    downside_deviation: number;
    value_at_risk_95: number;
    expected_shortfall_95: number;
    /** v10.1: Sharpe over log daily returns. */
    sharpe_ratio_log?: number | null;
}

export interface PerformanceMatrixRow {
    setup_type: string;
    regime: string;
    trade_count: number;
    win_rate: number;
    profit_factor: number | null;
    avg_r_multiple: number;
    total_pnl: number;
    compatibility_label: 'Strong' | 'Favorable' | 'Marginal' | 'Avoid';
}

export interface OptimizationReport {
    timestamp: number;
    total_trades: number;
    regime_reports: RegimePerformanceReport[];
    recommendations: string[];
}

export interface RegimePerformanceReport {
    regime: string;
    trade_count: number;
    win_rate: number;
    profit_factor: number;
    avg_r_multiple: number;
    total_pnl: number;
}

export interface TradeAnalyticsRecord {
    trade_id: string;
    symbol: string;
    direction: string;
    entry_timestamp: number;
    exit_timestamp: number;
    hold_time_seconds: number;
    entry_price: number;
    exit_price: number;
    size: number;
    gross_pnl: number;
    net_pnl: number;
    roi_pct: number;
    execution_slippage: number;
    mfe: number;
    mae: number;
    trigger_source: string;
    exit_reason: string;
    flat_trade: boolean;
}
