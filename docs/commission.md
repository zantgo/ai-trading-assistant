# Commission & Fee Modeling

> Reference spec for the commission calculation engine. Code: `crates/engine/src/commission.rs`.

---

## Purpose

The commission module computes all trade-related costs: maker/taker fees, funding rate decay, and viability gating. It provides fee tables for manual entry sizing and dual-entry projections for the 3-layer scaling model used in IEPL.

---

## Key Structures

### FeeTableRow
| Field | Type | Description |
|-------|------|-------------|
| `exchange_fee_pct` | f64 | Maker or taker fee as percentage |
| `leverage` | u32 | Position leverage |
| `capital` | f64 | Allocated capital |
| `min_profit_pct_to_cover_fees` | f64 | Break-even profit % accounting for round-trip fees |
| `fees_in_dollars` | f64 | Absolute fee cost in USD |

### FeeBreakdown
| Field | Type | Description |
|-------|------|-------------|
| `maker_fee_pct` | f64 | Maker fee rate |
| `taker_fee_pct` | f64 | Taker fee rate |
| `order_type` | String | "LIMIT" or "MARKET" |
| `effective_fee_pct` | f64 | Blended rate based on order type |
| `entry_1_fees` | f64 | Fees for Entry 1 |
| `entry_2_fees` | f64 | Fees for Entry 2 |
| `total_fees` | f64 | Sum of all entry fees |
| `funding_rate_8h` | f64 | 8-hour funding rate (from config) |
| `funding_cost` | f64 | Projected funding decay cost |

### CommissionProjection
Full projection for a dual-entry position:
- Direction, leverage, total capital
- Weighted average entry, effective stop/take-profit
- Risk amount, potential profit, max gain/loss net of fees
- **Viability Gate:** `trade_viable` = `max_gain_net_after_fees > 0`

---

## Funding Rate Decay

The `run_funding_decay_tracker` (in `paper_trading.rs`) runs on an 8-hour interval per active paper position. Each active slot incurs:
```
funding_cost = slot_size × current_price × funding_rate_8h
```
This is deducted from the slot's accumulated margin. Negative funding rates (shorts paying longs) are supported.

---

## Configuration

```toml
[fees]
maker_fee_pct = 0.02      # Hyperliquid maker fee
taker_fee_pct = 0.06      # Hyperliquid taker fee
funding_rate_8h = 0.01    # Manual default (overridden by live WS in Phase 11)
```

---

## Integration

- **Paper Trading:** Fees deducted from realized PnL on every slot close
- **Risk Calculator:** `risk_calculator.rs` includes funding cost in position projections
- **Frontend:** Commission calculator UI in the Settings panel
- **IEPL:** Viability gate — trades where max gain after fees ≤ 0 are rejected
