// Dashboard Statistics & Ledger Types
// Maps to Rust stats_compiler.rs DashboardStats struct

export interface CoreStats {
    total_pnl: number;
    win_rate: number;
    avg_loss: number;
    avg_gain: number;
    expectancy: number;
    avg_risk_reward_ratio: number;
    profit_factor: number;
    largest_loss: number;
    largest_gain: number;
    total_trades: number;
    wins: number;
    losses: number;
}

export interface DailyActivity {
    date: string;
    longs: number;
    shorts: number;
    win_rate: number;
}

export interface DailyPnl { date: string; pnl: number; }
export interface HourlyWinRate { hour: number; win_rate: number; volume: number; }
export interface WeekdayWinRate { weekday: string; win_rate: number; volume: number; }
export interface DirectionBreakdown {
    longs: number;
    shorts: number;
    long_expectancy: number;
    short_expectancy: number;
    long_wins: number;
    long_losses: number;
    long_win_rate: number;
    long_avg_gain: number;
    long_avg_loss: number;
    short_wins: number;
    short_losses: number;
    short_win_rate: number;
    short_avg_gain: number;
    short_avg_loss: number;
}
export interface StyleSegment { count: number; avg_duration_minutes: number; win_rate: number; }
export interface TraderStyleBreakdown { scalper: StyleSegment; day_trader: StyleSegment; swing_trader: StyleSegment; }
export interface StreakMetrics { avg_streak_length: number; max_consecutive_value: number; max_streak_length: number; }
export interface CalendarDay { date: string; pnl: number; month: number; day: number; }
export interface PairStat { symbol: string; value: number; }
export interface DailyCommission { date: string; fees: number; }
export interface FeePnlRatio { date: string; ratio: number; }
export interface MonthlySummary { month: string; net_pnl: number; win_rate: number; trade_count: number; }

export interface DashboardStats {
    core_stats: CoreStats;
    equity_curve: [number, number][];
    compounded_curve: [number, number][];
    daily_activity: DailyActivity[];
    daily_pnl: DailyPnl[];
    win_rate_by_hour: HourlyWinRate[];
    win_rate_by_weekday: WeekdayWinRate[];
    direction_breakdown: DirectionBreakdown;
    trader_style: TraderStyleBreakdown;
    winning_streaks: StreakMetrics;
    losing_streaks: StreakMetrics;
    post_loss_recovery_pct: number;
    pnl_calendar: CalendarDay[];
    pair_volume: PairStat[];
    top_pairs_profitability: PairStat[];
    bottom_pairs_profitability: PairStat[];
    daily_commissions: DailyCommission[];
    cumulative_commissions: [number, number][];
    fee_pnl_ratio: FeePnlRatio[];
    monthly_summary: MonthlySummary[];
}

export interface TradeLedgerRecord {
    id: number;
    exchange: string;
    symbol: string;
    direction: string;
    entry_timestamp: number;
    exit_timestamp: number;
    entry_price: number;
    exit_price: number;
    size: number;
    commission_fees: number;
    funding_fees: number;
    realized_pnl: number;
    roi_percentage: number;
    trigger_source: string;
}

export interface TradeJournalRecord {
    id: number;
    trade_id: number;
    entry_date: string;
    exit_date: string;
    asset: string;
    direction: string;
    entry_reason: string;
    roe_percentage: number;
    final_analysis: string;
    execution_score: number;
    human_notes: string;
    created_at: string;
    symbol: string;
    realized_pnl: number;
    roi_percentage: number;
}
