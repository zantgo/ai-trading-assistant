use sqlx::SqlitePool;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct DashboardStats {
    pub core_stats: CoreStats,
    pub equity_curve: Vec<(i64, f64)>,
    pub daily_activity: Vec<DailyActivity>,
    pub daily_pnl: Vec<DailyPnl>,
    pub win_rate_by_hour: Vec<HourlyWinRate>,
    pub win_rate_by_weekday: Vec<WeekdayWinRate>,
    pub direction_breakdown: DirectionBreakdown,
    pub trader_style: TraderStyleBreakdown,
    pub winning_streaks: StreakMetrics,
    pub losing_streaks: StreakMetrics,
    pub post_loss_recovery_pct: f64,
    pub pnl_calendar: Vec<CalendarDay>,
    pub pair_volume: Vec<PairStat>,
    pub top_pairs_profitability: Vec<PairStat>,
    pub bottom_pairs_profitability: Vec<PairStat>,
    pub daily_commissions: Vec<DailyCommission>,
    pub cumulative_commissions: Vec<(i64, f64)>,
    pub fee_pnl_ratio: Vec<FeePnlRatio>,
    pub monthly_summary: Vec<MonthlySummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CoreStats {
    pub total_pnl: f64,
    pub win_rate: f64,
    pub avg_loss: f64,
    pub avg_gain: f64,
    pub expectancy: f64,
    pub avg_risk_reward_ratio: f64,
    pub largest_loss: f64,
    pub largest_gain: f64,
    pub total_trades: usize,
    pub wins: usize,
    pub losses: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct DailyActivity {
    pub date: String,
    pub longs: usize,
    pub shorts: usize,
    pub win_rate: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DailyPnl {
    pub date: String,
    pub pnl: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct HourlyWinRate {
    pub hour: u32,
    pub win_rate: f64,
    pub volume: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct WeekdayWinRate {
    pub weekday: String,
    pub win_rate: f64,
    pub volume: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct DirectionBreakdown {
    pub longs: usize,
    pub shorts: usize,
    pub long_expectancy: f64,
    pub short_expectancy: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TraderStyleBreakdown {
    pub scalper: StyleSegment,
    pub day_trader: StyleSegment,
    pub swing_trader: StyleSegment,
}

#[derive(Debug, Clone, Serialize)]
pub struct StyleSegment {
    pub count: usize,
    pub avg_duration_minutes: f64,
    pub win_rate: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct StreakMetrics {
    pub avg_streak_length: f64,
    pub max_consecutive_value: f64,
    pub max_streak_length: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct CalendarDay {
    pub date: String,
    pub pnl: f64,
    pub month: u32,
    pub day: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct PairStat {
    pub symbol: String,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DailyCommission {
    pub date: String,
    pub fees: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct FeePnlRatio {
    pub date: String,
    pub ratio: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct MonthlySummary {
    pub month: String,
    pub net_pnl: f64,
    pub win_rate: f64,
    pub trade_count: usize,
}

pub async fn compile_dashboard_stats(pool: &SqlitePool) -> DashboardStats {
    let trades: Vec<(i64, String, String, f64, f64, f64, f64, f64, f64, String)> =
        crate::db::dash_trade_detail(pool).await;

    if trades.is_empty() {
        return empty_dashboard();
    }

    let core_stats = compute_core_stats(&trades);
    let equity_curve = compute_equity_curve(&trades);
    let daily_activity = compute_daily_activity(&trades);
    let daily_pnl = compute_daily_pnl(&trades);
    let win_rate_by_hour = compute_win_rate_by_hour(&trades);
    let win_rate_by_weekday = compute_win_rate_by_weekday(&trades);
    let direction_breakdown = compute_direction_breakdown(&trades);
    let trader_style = compute_trader_style(&trades);
    let (winning_streaks, losing_streaks, post_loss_recovery_pct) = compute_streaks(&trades);
    let pnl_calendar = compute_pnl_calendar(&trades);
    let pair_volume = compute_pair_volume(&trades);
    let (top_pairs_profitability, bottom_pairs_profitability) = compute_pair_profitability(&trades);
    let (daily_commissions, cumulative_commissions, fee_pnl_ratio) = compute_commission_stats(&trades);
    let monthly_summary = compute_monthly_summary(&trades);

    DashboardStats {
        core_stats,
        equity_curve,
        daily_activity,
        daily_pnl,
        win_rate_by_hour,
        win_rate_by_weekday,
        direction_breakdown,
        trader_style,
        winning_streaks,
        losing_streaks,
        post_loss_recovery_pct,
        pnl_calendar,
        pair_volume,
        top_pairs_profitability,
        bottom_pairs_profitability,
        daily_commissions,
        cumulative_commissions,
        fee_pnl_ratio,
        monthly_summary,
    }
}

fn empty_dashboard() -> DashboardStats {
    DashboardStats {
        core_stats: CoreStats {
            total_pnl: 0.0, win_rate: 0.0, avg_loss: 0.0, avg_gain: 0.0,
            expectancy: 0.0, avg_risk_reward_ratio: 0.0,
            largest_loss: 0.0, largest_gain: 0.0,
            total_trades: 0, wins: 0, losses: 0,
        },
        equity_curve: vec![],
        daily_activity: vec![],
        daily_pnl: vec![],
        win_rate_by_hour: vec![],
        win_rate_by_weekday: vec![],
        direction_breakdown: DirectionBreakdown { longs: 0, shorts: 0, long_expectancy: 0.0, short_expectancy: 0.0 },
        trader_style: TraderStyleBreakdown {
            scalper: StyleSegment { count: 0, avg_duration_minutes: 0.0, win_rate: 0.0 },
            day_trader: StyleSegment { count: 0, avg_duration_minutes: 0.0, win_rate: 0.0 },
            swing_trader: StyleSegment { count: 0, avg_duration_minutes: 0.0, win_rate: 0.0 },
        },
        winning_streaks: StreakMetrics { avg_streak_length: 0.0, max_consecutive_value: 0.0, max_streak_length: 0 },
        losing_streaks: StreakMetrics { avg_streak_length: 0.0, max_consecutive_value: 0.0, max_streak_length: 0 },
        post_loss_recovery_pct: 0.0,
        pnl_calendar: vec![],
        pair_volume: vec![],
        top_pairs_profitability: vec![],
        bottom_pairs_profitability: vec![],
        daily_commissions: vec![],
        cumulative_commissions: vec![],
        fee_pnl_ratio: vec![],
        monthly_summary: vec![],
    }
}

type TradeRow = (i64, String, String, f64, f64, f64, f64, f64, f64, String);
// (exit_ts, symbol, direction, entry_price, exit_price, size, realized_pnl, commission_fees, roi_pct, trigger)

fn compute_core_stats(trades: &[TradeRow]) -> CoreStats {
    let total = trades.len();
    let wins: Vec<&TradeRow> = trades.iter().filter(|t| t.6 > 0.0).collect();
    let losses: Vec<&TradeRow> = trades.iter().filter(|t| t.6 < 0.0).collect();

    let total_pnl: f64 = trades.iter().map(|t| t.6).sum();
    let win_rate = if total > 0 { wins.len() as f64 / total as f64 } else { 0.0 };

    let avg_gain = if wins.is_empty() { 0.0 } else { wins.iter().map(|t| t.6).sum::<f64>() / wins.len() as f64 };
    let avg_loss = if losses.is_empty() { 0.0 } else { losses.iter().map(|t| t.6.abs()).sum::<f64>() / losses.len() as f64 };

    let expectancy = if total > 0 {
        (win_rate * avg_gain) - ((1.0 - win_rate) * avg_loss)
    } else { 0.0 };

    let largest_gain = trades.iter().map(|t| t.6).fold(0.0, f64::max);
    let largest_loss = trades.iter().map(|t| t.6).fold(0.0, f64::min);

    let avg_rr = if !wins.is_empty() && !losses.is_empty() {
        let win_roi = wins.iter().map(|t| t.8.abs()).sum::<f64>() / wins.len() as f64;
        let loss_roi = losses.iter().map(|t| t.8.abs()).sum::<f64>() / losses.len() as f64;
        if loss_roi > 0.0 { win_roi / loss_roi } else { 0.0 }
    } else { 0.0 };

    CoreStats {
        total_pnl, win_rate, avg_loss, avg_gain, expectancy, avg_risk_reward_ratio: avg_rr,
        largest_loss, largest_gain,
        total_trades: total, wins: wins.len(), losses: losses.len(),
    }
}

fn compute_equity_curve(trades: &[TradeRow]) -> Vec<(i64, f64)> {
    let mut cumulative = 0.0;
    trades.iter().map(|t| {
        cumulative += t.6;
        (t.0, cumulative)
    }).collect()
}

fn compute_daily_activity(trades: &[TradeRow]) -> Vec<DailyActivity> {
    use std::collections::BTreeMap;
    let mut groups: BTreeMap<String, (usize, usize, usize)> = BTreeMap::new();
    for t in trades {
        let date = format_ts_date(t.0);
        let entry = groups.entry(date).or_insert((0, 0, 0));
        if t.2.to_uppercase() == "LONG" { entry.0 += 1; } else { entry.1 += 1; }
        if t.6 > 0.0 { entry.2 += 1; }
    }
    groups.into_iter().map(|(date, (longs, shorts, wins))| {
        let total = longs + shorts;
        DailyActivity { date, longs, shorts, win_rate: if total > 0 { wins as f64 / total as f64 } else { 0.0 } }
    }).collect()
}

fn compute_daily_pnl(trades: &[TradeRow]) -> Vec<DailyPnl> {
    use std::collections::BTreeMap;
    let mut groups: BTreeMap<String, f64> = BTreeMap::new();
    for t in trades {
        *groups.entry(format_ts_date(t.0)).or_insert(0.0) += t.6;
    }
    groups.into_iter().map(|(date, pnl)| DailyPnl { date, pnl }).collect()
}

fn compute_win_rate_by_hour(trades: &[TradeRow]) -> Vec<HourlyWinRate> {
    let mut hours = vec![vec![0_usize; 2]; 24];
    for t in trades {
        let h = ts_to_hour(t.0);
        if h < 24 {
            hours[h][0] += 1;
            if t.6 > 0.0 { hours[h][1] += 1; }
        }
    }
    hours.iter().enumerate().map(|(h, counts)| {
        let total = counts[0];
        HourlyWinRate {
            hour: h as u32,
            win_rate: if total > 0 { counts[1] as f64 / total as f64 } else { 0.0 },
            volume: total,
        }
    }).collect()
}

fn compute_win_rate_by_weekday(trades: &[TradeRow]) -> Vec<WeekdayWinRate> {
    const NAMES: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    let mut days = vec![vec![0_usize; 2]; 7];
    for t in trades {
        let wd = ts_to_weekday(t.0);
        if wd < 7 {
            days[wd][0] += 1;
            if t.6 > 0.0 { days[wd][1] += 1; }
        }
    }
    days.iter().enumerate().map(|(d, counts)| {
        let total = counts[0];
        WeekdayWinRate {
            weekday: NAMES[d].to_string(),
            win_rate: if total > 0 { counts[1] as f64 / total as f64 } else { 0.0 },
            volume: total,
        }
    }).collect()
}

fn compute_direction_breakdown(trades: &[TradeRow]) -> DirectionBreakdown {
    let longs: Vec<_> = trades.iter().filter(|t| t.2.to_uppercase() == "LONG").collect();
    let shorts: Vec<_> = trades.iter().filter(|t| t.2.to_uppercase() == "SHORT").collect();
    let long_exp = if !longs.is_empty() { longs.iter().map(|t| t.6).sum::<f64>() / longs.len() as f64 } else { 0.0 };
    let short_exp = if !shorts.is_empty() { shorts.iter().map(|t| t.6).sum::<f64>() / shorts.len() as f64 } else { 0.0 };
    DirectionBreakdown { longs: longs.len(), shorts: shorts.len(), long_expectancy: long_exp, short_expectancy: short_exp }
}

fn compute_trader_style(trades: &[TradeRow]) -> TraderStyleBreakdown {
    let mut scalper = Vec::new();
    let mut day_trader = Vec::new();
    let mut swing = Vec::new();
    for t in trades {
        let dur_min = (t.4 - t.3).max(0.0) / 60.0; // duration in minutes
        let dur_entry = (dur_min, t.6);
        if dur_min <= 30.0 { scalper.push(dur_entry); }
        else if dur_min <= 1440.0 { day_trader.push(dur_entry); }
        else { swing.push(dur_entry); }
    }
    fn build_seg(data: &[(f64, f64)]) -> StyleSegment {
        if data.is_empty() { return StyleSegment { count: 0, avg_duration_minutes: 0.0, win_rate: 0.0 }; }
        let count = data.len();
        let avg_dur = data.iter().map(|d| d.0).sum::<f64>() / count as f64;
        let wins = data.iter().filter(|d| d.1 > 0.0).count();
        StyleSegment { count, avg_duration_minutes: avg_dur, win_rate: wins as f64 / count as f64 }
    }
    TraderStyleBreakdown {
        scalper: build_seg(&scalper),
        day_trader: build_seg(&day_trader),
        swing_trader: build_seg(&swing),
    }
}

fn compute_streaks(trades: &[TradeRow]) -> (StreakMetrics, StreakMetrics, f64) {
    let mut win_streaks = vec![];
    let mut loss_streaks = vec![];
    let mut cur_win_streak = 0;
    let mut cur_loss_streak = 0;
    let mut cur_win_val = 0.0;
    let mut cur_loss_val = 0.0;

    for t in trades {
        if t.6 > 0.0 {
            cur_win_streak += 1;
            cur_win_val += t.6;
            if cur_loss_streak > 0 {
                loss_streaks.push((cur_loss_streak, cur_loss_val));
                cur_loss_streak = 0;
                cur_loss_val = 0.0;
            }
        } else if t.6 < 0.0 {
            cur_loss_streak += 1;
            cur_loss_val += t.6;
            if cur_win_streak > 0 {
                win_streaks.push((cur_win_streak, cur_win_val));
                cur_win_streak = 0;
                cur_win_val = 0.0;
            }
        }
    }
    if cur_win_streak > 0 { win_streaks.push((cur_win_streak, cur_win_val)); }
    if cur_loss_streak > 0 { loss_streaks.push((cur_loss_streak, cur_loss_val)); }

    fn build(streaks: &[(usize, f64)]) -> StreakMetrics {
        if streaks.is_empty() { return StreakMetrics { avg_streak_length: 0.0, max_consecutive_value: 0.0, max_streak_length: 0 }; }
        let avg_len = streaks.iter().map(|s| s.0 as f64).sum::<f64>() / streaks.len() as f64;
        let max_val = streaks.iter().map(|s| s.1).fold(0.0, f64::max);
        let max_len = streaks.iter().map(|s| s.0).max().unwrap_or(0);
        StreakMetrics { avg_streak_length: avg_len, max_consecutive_value: max_val, max_streak_length: max_len }
    }

    let mut post_loss_recovery = 0;
    let mut post_loss_opportunities = 0;
    for i in 1..trades.len() {
        if trades[i - 1].6 < 0.0 {
            post_loss_opportunities += 1;
            if trades[i].6 > 0.0 { post_loss_recovery += 1; }
        }
    }
    let recovery_pct = if post_loss_opportunities > 0 {
        (post_loss_recovery as f64 / post_loss_opportunities as f64) * 100.0
    } else { 0.0 };

    (build(&win_streaks), build(&loss_streaks), recovery_pct)
}

fn compute_pnl_calendar(trades: &[TradeRow]) -> Vec<CalendarDay> {
    use std::collections::BTreeMap;
    let mut days: BTreeMap<String, f64> = BTreeMap::new();
    for t in trades {
        let date = format_ts_date(t.0);
        *days.entry(date).or_insert(0.0) += t.6;
    }
    days.into_iter().map(|(date, pnl)| {
        let parts: Vec<&str> = date.split('-').collect();
        let month = parts.get(1).and_then(|m| m.parse().ok()).unwrap_or(0);
        let day = parts.get(2).and_then(|d| d.parse().ok()).unwrap_or(0);
        CalendarDay { date, pnl, month, day }
    }).collect()
}

fn compute_pair_volume(trades: &[TradeRow]) -> Vec<PairStat> {
    aggregate_by_symbol(trades, |_| 1.0)
}

fn compute_pair_profitability(trades: &[TradeRow]) -> (Vec<PairStat>, Vec<PairStat>) {
    let mut all = aggregate_by_symbol(trades, |t| t.6);
    all.sort_by(|a, b| b.value.partial_cmp(&a.value).unwrap_or(std::cmp::Ordering::Equal));
    let top = all.iter().take(6).cloned().collect();
    let bottom: Vec<PairStat> = all.iter().rev().take(6).cloned().collect();
    (top, bottom)
}

fn aggregate_by_symbol(trades: &[TradeRow], f: fn(&TradeRow) -> f64) -> Vec<PairStat> {
    use std::collections::BTreeMap;
    let mut map: BTreeMap<String, f64> = BTreeMap::new();
    for t in trades {
        *map.entry(t.1.clone()).or_insert(0.0) += f(t);
    }
    map.into_iter().map(|(symbol, value)| PairStat { symbol, value }).collect()
}

fn compute_commission_stats(trades: &[TradeRow]) -> (Vec<DailyCommission>, Vec<(i64, f64)>, Vec<FeePnlRatio>) {
    use std::collections::BTreeMap;
    let mut daily: BTreeMap<String, (f64, f64)> = BTreeMap::new();
    for t in trades {
        let date = format_ts_date(t.0);
        let entry = daily.entry(date).or_insert((0.0, 0.0));
        entry.0 += t.7;
        entry.1 += t.6;
    }
    let daily_commissions: Vec<DailyCommission> = daily.iter().map(|(date, (fees, _))| {
        DailyCommission { date: date.clone(), fees: *fees }
    }).collect();

    let mut cum = 0.0;
    let cumulative: Vec<(i64, f64)> = trades.iter().map(|t| { cum += t.7; (t.0, cum) }).collect();

    let fee_pnl: Vec<FeePnlRatio> = daily.iter().map(|(date, (fees, pnl))| {
        let ratio = if pnl.abs() > 0.0 { (fees / pnl.abs()) * 100.0 } else { 0.0 };
        FeePnlRatio { date: date.clone(), ratio }
    }).collect();

    (daily_commissions, cumulative, fee_pnl)
}

fn compute_monthly_summary(trades: &[TradeRow]) -> Vec<MonthlySummary> {
    use std::collections::BTreeMap;
    let mut months: BTreeMap<String, (f64, usize, usize)> = BTreeMap::new();
    for t in trades {
        let month = format_ts_month(t.0);
        let entry = months.entry(month).or_insert((0.0, 0, 0));
        entry.0 += t.6;
        entry.1 += 1;
        if t.6 > 0.0 { entry.2 += 1; }
    }
    months.into_iter().map(|(month, (pnl, total, wins))| {
        MonthlySummary { month, net_pnl: pnl, win_rate: if total > 0 { wins as f64 / total as f64 } else { 0.0 }, trade_count: total }
    }).collect()
}

fn format_ts_date(ts: i64) -> String {
    let secs = if ts > 9_000_000_000 { ts / 1000 } else { ts };
    let days_since_epoch = secs / 86400;
    let year = 1970 + (days_since_epoch / 365);
    let day_of_year = days_since_epoch % 365;
    let mut month = 0;
    let months_days = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut remaining = day_of_year;
    for (i, &md) in months_days.iter().enumerate() {
        if remaining < md { month = i + 1; break; }
        remaining -= md;
        month = i + 1;
    }
    let day = remaining + 1;
    format!("{:04}-{:02}-{:02}", year, month, day)
}

fn format_ts_month(ts: i64) -> String {
    let date = format_ts_date(ts);
    date[..7].to_string()
}

fn ts_to_hour(ts: i64) -> usize {
    let secs = if ts > 9_000_000_000 { ts / 1000 } else { ts };
    ((secs % 86400) / 3600) as usize
}

fn ts_to_weekday(ts: i64) -> usize {
    let secs = if ts > 9_000_000_000 { ts / 1000 } else { ts };
    ((secs / 86400 + 4) % 7) as usize
}
