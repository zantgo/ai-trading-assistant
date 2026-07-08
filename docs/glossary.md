# Institutional Trading Glossary

> Reference for all technical terms, acronyms, abbreviations, and layer initials used
> across the AI Trading Assistant documentation suite.

---

## Layer Initials

| Initials | Full Name | One-Line Definition |
|----------|-----------|---------------------|
| **ITIL** | Institutional Technical Indicator Layer | 51 indicators across 7 groups with 115 signal emissions; transforms OHLCV into normalized [-1,+1] values and discrete signal events |
| **IRCL** | Institutional Regime Classification Layer | Classifies market into 5 regimes (Trending, Compression, Expansion, Range, Transitional) weighted across 6 voting indicators |
| **ISML** | Institutional Structure Mapping Layer | Maps the battlefield: S/R with role tracking, Fibonacci zones, Volume Profile, Smart Money Concepts, chart patterns |
| **ICSL** | Institutional Confluence Scoring Layer | Synthesizes 44 directional indicators into a weighted [-100,+100] score gated by 7 non-directional multipliers |
| **IDCL** | Institutional Decision Context Layer | 17 quantitative decision-support metrics: probability, consensus, range, volatility, risk, quality, readiness |
| **ISIL** | Institutional Statistical Intelligence Layer | 6 modules (A-F) + ML: distribution statistics, empirical probabilities, confidence intervals, market shape, correlations, Monte Carlo |
| **IRML** | Institutional Risk Management Layer | 6 risk categories + position profile + capital allocation + drawdown state machine + adaptive R:R engine |
| **IASL** | Institutional AI Synthesis Layer | Two-agent pipeline: Analyst Agent prepares institutional document from all deterministic data; Trader Agent makes final decision |
| **IEPL** | Institutional Execution Protocol Layer | Entry protocol (3-layer scaling), fractional slot machine (4 FIFO slots), stop/target placement, invalidation, trailing |
| **IPEL** | Institutional Performance Evaluation Layer | Trade journaling, post-trade audit, performance metrics, direction correctness, historical analysis, adaptive learning feedback |
| **IUSF** | Institutional Unified Strategy Framework | High-level strategy philosophy document summarizing all 10 layers with cross-references |

---

## Smart Money Concepts (SMC)

| Term | Definition |
|------|------------|
| **BOS** | Break of Structure — price makes a higher high (bullish) or lower low (bearish), confirming the current trend structure |
| **CHoCH** | Change of Character — price makes a lower high in a bull trend or higher low in a bear trend, warning of potential trend reversal or range shift |
| **FVG** | Fair Value Gap — 3-candle pattern where Low[3] > High[1] (bullish) or High[3] < Low[1] (bearish); imbalance zone acting as a price magnet |
| **OB** | Order Block — last opposing candle before a BOS; bullish OB = last bearish candle before bullish BOS (demand zone), bearish OB = last bullish candle before bearish BOS (supply zone) |
| **Liquidity Sweep** | Price wicks beyond a recent swing level and closes back inside; buy-side sweep = stops below swing low hunted (bullish), sell-side sweep = stops above swing high hunted (bearish) |
| **Mitigated FVG** | Price has traded through the gap — the imbalance is resolved. A mitigated bullish FVG that later holds as support becomes an inverse FVG |
| **Mitigated OB** | Price closes beyond the order block zone — the block is broken. Mitigated bullish OB becomes potential resistance; mitigated bearish OB becomes potential support |

---

## Volume Profile Terms

| Term | Definition |
|------|------------|
| **POC** | Point of Control — the price level with the highest traded volume in the profile window; acts as a magnet level |
| **VAH** | Value Area High — the upper boundary of the Value Area (typically 70% of total volume); price above VAH = bullish breakout |
| **VAL** | Value Area Low — the lower boundary of the Value Area; price below VAL = bearish breakdown |
| **HVN** | High Volume Node — price cluster with significantly elevated volume; acts as support/resistance |
| **LVN** | Low Volume Node — price cluster with minimal volume; price moves quickly through these zones |
| **Value Area** | The price range containing ~70% of total volume, bounded by VAH and VAL |
| **Value Acceptance** | Price trading within the value area — equilibrium, trending less likely |
| **Value Rejection** | Price closing outside value area after being inside — directional commitment |

---

## Indicator Acronyms

| Acronym | Full Name | Type |
|---------|-----------|------|
| **ADX** | Average Directional Index | Trend strength (non-directional gate) |
| **AO** | Awesome Oscillator | Momentum (SMA5 − SMA34 of median price) |
| **ATR** | Average True Range | Volatility measure (non-directional gate) |
| **BB** | Bollinger Bands | Volatility envelope (±2σ around SMA20) |
| **BBWP** | Bollinger Band Width Percentile | Volatility compression/expansion percentile |
| **CCI** | Commodity Channel Index | Momentum oscillator (deviation from statistical mean) |
| **CMF** | Chaikin Money Flow | Volume-weighted accumulation/distribution |
| **CMO** | Chande Momentum Oscillator | Raw momentum ratio of gains vs losses |
| **EMA** | Exponential Moving Average | Weighted moving average (10, 50, 100, 200 periods) |
| **FI** | Force Index | (Close − PrevClose) × Volume, EMA-smoothed |
| **HMA** | Hull Moving Average | Near-zero-lag weighted moving average |
| **HV** | Historical Volatility | Annualized standard deviation of log returns (non-directional gate) |
| **MACD** | Moving Average Convergence Divergence | Trend-following momentum oscillator (12/26/9) |
| **MFI** | Money Flow Index | Volume-weighted RSI [0,100] |
| **OBV** | On-Balance Volume | Cumulative volume signed by close direction |
| **PSAR** | Parabolic Stop and Reverse | Trailing stop-loss dot; flips on reversal |
| **RSI** | Relative Strength Index | Momentum oscillator [0,100] using Wilder's smoothing |
| **RVOL** | Relative Volume | Current volume / average volume (non-directional gate) |
| **SMA** | Simple Moving Average | Arithmetic mean of price over N periods |
| **S/R** | Support / Resistance | Horizontal price levels where supply/demand shift |
| **VWAP** | Volume Weighted Average Price | Intraday average price weighted by cumulative volume |

---

## Statistical Terms

| Term | Definition |
|------|------------|
| **Entropy** | Shannon entropy of price return histogram (normalized [0,1]); >0.7 = random/unpredictable, <0.3 = highly structured |
| **Kurtosis** | Fourth standardized moment − 3; >2 = fat tails (chaotic), >3 = extreme |
| **Skewness** | Third standardized moment; >0.5 = bullish tail risk, <−0.5 = bearish tail risk |
| **Z-Score** | (current − mean) / σ; number of standard deviations from the rolling mean |
| **Percentile** | Rank of current value in historical distribution [0,100] |
| **IQR** | Interquartile Range — Q3 − Q1 |
| **MAD** | Median Absolute Deviation — median(|xi − median|) |
| **HDI** | Highest Density Interval — Bayesian credible interval of the posterior distribution |
| **Mahalanobis Distance** | Multivariate distance from mean accounting for covariance; used for anomaly detection |
| **Welford's Algorithm** | Incremental variance computation in O(1) per observation |
| **Beta-Binomial Conjugate** | Bayesian model for probability estimation; prior Beta(α,β) + observations → posterior Beta(α+successes, β+failures) |

---

## Signal Kind Definitions

| SignalKind | Definition | Example |
|------------|------------|---------|
| **Divergence** | Oscillator disagrees with price direction | RSI makes higher low while price makes lower low |
| **Crossover** | Two lines cross | EMA 10 crosses above EMA 50; MACD crosses signal line |
| **Threshold** | Value crosses a predefined boundary | RSI > 70 (overbought), ADX < 20 (congestion) |
| **Breakout** | Price closes beyond a channel/band/envelope | Close above Donchian upper; close above Bollinger upper |
| **BandTouch** | Price touches a band without closing beyond it | Wick touches Keltner lower but close is inside |
| **ZeroLineCross** | Oscillator crosses the zero line | MACD histogram goes from negative to positive |
| **CompressionRelease** | Volatility compression releases | Squeeze turns OFF (red dots → green dots) |
| **LevelTest** | Price tests a structural level | Price approaches Fibonacci 0.618; price nears VWAP |
| **TrendFlip** | Direction changes | Supertrend flips from bullish to bearish; PSAR dot flips |
| **VolumeClimax** | Extreme volume event | RVOL ≥ 3.0 — exhaustion climax |
| **StackChange** | EMA ribbon ordering changes | EMA 10 drops below EMA 50 (bearish reorder) |
| **PatternForming** | Candlestick or chart pattern detected | Hammer, engulfing, triangle breakout |

---

## Regime Terms

| Term | Definition |
|------|------------|
| **Trending** | ADX > 25, BBWP expanding, price respecting EMA structure; highest allocation permitted |
| **Compression** | BBWP < 20th percentile, Squeeze active, ATR contracting; breakout preparation only |
| **Expansion** | BBWP rapidly increasing, ATR expanding, Squeeze recently released; highest-probability environment |
| **Range** | ADX < 20, flat EMA structure, price oscillating between S/R; trend-following prohibited |
| **Transitional** | Regime changed within recent bars; 3+ indicators shifting; avoid directional bets |

---

## Execution & Position Terms

| Term | Definition |
|------|------------|
| **FIFO** | First-In, First-Out — oldest slot closed first on partial exit |
| **Slot Machine** | Fractional Dynamic Position Slot Machine — 4 discrete slots managing leveraged positions portion-by-portion |
| **C_cycle** | Total cycle capital = initial margin + realized PnL accumulator |
| **U_cycle** | Unallocated cycle capital = C_cycle − Σ(active slot margins) |
| **Breakeven Trail** | After TP1 hit, stop-loss moved to weighted average entry price for remaining slots |
| **Decisive Close** | 1-minute candle close beyond invalidation level with 0.2% tolerance buffer; rejects wicks |
| **Golden Pocket** | Fibonacci 61.8%–66.0% retracement zone — highest-probability institutional accumulation/distribution area |
| **R:R** | Reward/Risk ratio — target profit multiple relative to risk unit; R_recommended = k × (1−W)/W |
| **Reduce-Only Order** | Order that only reduces existing position, never increases it; used for TP/SL brackets |
| **Bracket Order** | Paired TP (limit) and SL (stop) orders attached to a specific slot |
| **Slippage** | Difference between expected execution price and actual fill price |

---

## API & System Terms

| Term | Definition |
|------|------------|
| **MarketSnapshot** | Complete per-candle data structure carrying OHLCV, 51 normalized indicators, MarketContext, DecisionContext, StatisticalContext, RiskProfile |
| **NormalizedIndicatorValue** | `{ raw_value, normalized[-1,+1], state_label, values{}, signals[], confidence }` per indicator per snapshot |
| **Registry Manifest** | `INDICATORS: &[IndicatorMeta]` — single source of truth for all 51 indicators; drives backend scoring, frontend rendering, AI context |
| **WAL** | Write-Ahead Logging — SQLite mode enabling concurrent reads during writes |
| **Two-Agent Pipeline** | Analyst Agent (information preparation) → Trader Agent (decision execution); sequential, not parallel |

---

## Risk Terms

| Term | Definition |
|------|------------|
| **Drawdown** | Percentage decline from equity high-water mark |
| **Expectancy** | Per-trade expected value: E = W·R − (1−W)·1 |
| **Profit Factor** | Σ gains / |Σ losses| |
| **Sharpe Ratio** | (mean return − risk-free rate) / σ of returns |
| **Sortino Ratio** | Like Sharpe but penalizes only downside deviation |
| **Recovery Factor** | Net profit / max drawdown |
| **Hysteresis** | Threshold gap preventing flapping; enter a risk level at higher score, exit at lower |
| **Trade Permission** | Final gate: Allowed / Restricted / High Caution / Suspended / Emergency Stop |
| **Exposure Tier** | Maximum / Normal / Reduced / Minimal / No Exposure — scaling factor on base allocation |
| **Beta Prior** | Bayesian prior encoding "5 wins out of 10" neutral assumption for R:R calibration; Beta(α=5, β=5) |
| **OI** | Open Interest — total number of outstanding derivative contracts. Rising OI + rising price = trend confirmation. Rising OI + falling price = bearish divergence |
| **OI Delta** | Change in Open Interest over a period (1h/4h/24h). Positive delta = new money entering; negative delta = positions being closed |
| **CVD** | Cumulative Volume Delta — cumulative sum of (buy volume − sell volume). Rising CVD = net buying pressure; falling CVD = net selling |
| **OFI** | Order Flow Imbalance — (bid depth − ask depth) / total depth. Range [-1,+1]. Positive = buy-side dominance; negative = sell-side |
| **DOM** | Depth of Market — order book depth at each price level. Measured in levels (50) or % depth (1%, 2%, 5% from mid) |
| **Kelly Criterion** | Position sizing formula: f* = W − (1−W)/R. Optimal fraction of bankroll. Half-Kelly (f*/2) used in practice |
| **Risk Parity** | Equal Risk Contribution — allocates weights w_i = (1/σ_i) / Σ(1/σ_j) so each asset contributes equal risk |
| **Funding Rate** | Periodic payment between long and short perp holders. Positive = longs pay shorts. Extreme positive → short signal; extreme negative → long signal |
| **Whale Wall** | Large resting limit order at a single price level. Act as temporary support/resistance; their removal signals directional intent |
