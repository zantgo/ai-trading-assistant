# Pre-Trade Risk Controls

**Version:** 0.1
**Status:** DRAFT — content TBD
**Category:** Operations & Compliance

> **DRAFT — content TBD.** This document is a labeled skeleton created alongside the workspace restructure. Section content has not yet been authored; headings below define the intended scope only. Do not treat any statement here as an implemented specification.

---

## 1. Purpose

_TBD — the mandatory checks applied between a validated policy trigger and order dispatch._

## 2. Control Catalogue

_TBD — enumerate pre-trade gates (max position notional, max leverage, margin sufficiency, slippage ceiling, symbol stance, PME safety veto). Cross-reference [TAE Layer 2 — Execution](../engines/trade-automation-engine/03-03-03-tae-layer2-execution.md) and [PME Layer 4 — Portfolio](../engines/portfolio-management-engine/03-04-05-pme-layer4-portfolio.md)._

## 3. Evaluation Order & Short-Circuiting

_TBD — deterministic order in which controls run and which are hard-stops vs. hold-for-review._

## 4. Parameters & Configuration

_TBD — where each control's thresholds live in `config.json` and per-policy overrides._

## 5. Rejections & Operator Overrides

_TBD — how a rejected order is logged and what manual confirmation is required to override._

---

## Cross-References

- [TAE Layer 2 — Execution](../engines/trade-automation-engine/03-03-03-tae-layer2-execution.md)
- [PME Layer 3 — Capital](../engines/portfolio-management-engine/03-04-04-pme-layer3-capital.md)
- [Regulatory Compliance & Audit](08-03-regulatory-compliance-and-audit.md)
