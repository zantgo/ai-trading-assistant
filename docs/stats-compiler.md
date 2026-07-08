# Stats Compiler & Performance Analytics

> Reference spec for the dashboard statistics aggregation engine. Code: `crates/engine/src/stats_compiler.rs`.

---

## Purpose

The stats compiler aggregates all trade telemetry and portfolio data into structured dashboard metrics. It is the data source for the frontend Performance Analytics tab, providing core stats, equity curves, daily breakdowns, hourly/weekday patterns, direction analysis, streak tracking, and regime-specific performance.

---

## Output Structure: `DashboardStats`

### CoreStats
| Metric | Description |
|--------|-------------|
| `total_pnl` | Aggregate realized P&L across all trades |
| `win_rate` | Wins / total trades |
| `avg_loss` | Mean realized loss |
| `avg_gain` | Mean realized gain |
| `expectancy` | Expected value per trade |
| `avg_risk_reward_ratio` | Mean realized reward multiple |
| `profit_factor` | Σ gains / |Σ losses| |
| `sharpe_ratio` | Risk-adjusted return |
| `max_drawdown_pct` | Peak-to-trough drawdown |
| `avg_trade_duration` | Mean hold time in seconds |
| `trade_frequency_hr` | Trades per hour |

### Time-Series Arrays
| Array | Content |
|-------|---------|
| `equity_curve` | (timestamp, equity) — cumulative P&L over time |
| `compounded_curve` | Compounded balance using ROI multipliers |
| `daily_activity` | Trade count + volume per day |
| `daily_pnl` | Aggregate P&L per day |
| `pnl_calendar` | Calendar-day P&L for heatmap |
| `daily_commissions` | Fee total per day |
| `cumulative_commissions` | Cumulative fee cost over time |
| `fee_pnl_ratio` | Fee-to-P&L ratio over time |

### Pattern Analytics
| Metric | Description |
|--------|-------------|
| `win_rate_by_hour` | Win rate by hour-of-day (UTC) |
| `win_rate_by_weekday` | Win rate by day of week |
| `direction_breakdown` | Long vs Short P&L, win rate, trade count |
| `trader_style` | Tag-based P&L attribution (LONG/SHORT/DIVERGENCE/etc.) |
| `winning_streaks` | Longest winning streak, distribution |
| `losing_streaks` | Longest losing streak, distribution |
| `post_loss_recovery_pct` | % of losses followed by profitable trades |

### Pair & Regime Analytics
| Metric | Description |
|--------|-------------|
| `pair_volume` | Trade volume per pair |
| `top_pairs_profitability` | Most profitable pairs ranked |
| `bottom_pairs_profitability` | Least profitable pairs ranked |
| `regime_breakdown` | Per-IRCL-regime: trades, wins, win rate, profit factor, total P&L, avg R |
| `monthly_summary` | Aggregate P&L, win rate, trades per month |

---

## Data Sources

- `paper_trades` — realized P&L, direction, ROI, timestamps
- `trade_telemetry_history` — per-slot execution mirror
- `portfolio_equity_history` — equity time-series for drawdown
- `trade_learning_journal` — journal agent scoring, tags

---

## Integration

- **Frontend:** Performance Analytics tab fetches `/api/performance` which calls `DashboardStats::compile`
- **IRML:** Win rate and performance metrics feed the Bayesian R:R engine
- **IPEL:** Regime breakdown identifies where strategy excels vs struggles
