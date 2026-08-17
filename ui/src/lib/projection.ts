// Project Risk and Return — shared state shape + pure helpers (v7.0).
//
// The Recommendation panel's "PROJECT RISK AND RETURN" drawer reads the
// backend `/api/risk/calculate` `RiskCalculation` and derives the
// operator-facing projection state. The same state shape is emitted by
// the recommendation tab export (`projection` block) so screen and
// clipboard can never disagree.

import type { RiskCalculation } from '../types';

/** Direction-adjusted setup geometry the drawer prefills from the active
 *  setup card (entry/take-profit midpoints, invalidation stop). */
export interface ProjectionSetup {
    direction: 'LONG' | 'SHORT';
    entry: number;
    stopLoss: number;
    takeProfit: number;
}

/**
 * Projection state — mirrors the drawer's output grid AND the export
 * block. `configured === false` renders the JSON block empty (null
 * numerics) until the operator actually runs a calculation.
 */
export interface ProjectionState {
    configured: boolean;
    capital: number | null;
    leverage: number | null;
    direction: 'LONG' | 'SHORT' | null;
    entry_price: number | null;
    stop_loss: number | null;
    take_profit: number | null;
    position_size_units: number | null;
    position_notional_usd: number | null;
    entry_fee_usd: number | null;
    exit_fee_usd: number | null;
    total_fees_usd: number | null;
    liquidation_price: number | null;
    net_profit_usd: number | null;
    roi_pct: number | null;
}

/** Default projection — `configured: false`, every numeric field null. */
export function emptyProjection(): ProjectionState {
    return {
        configured: false,
        capital: null,
        leverage: null,
        direction: null,
        entry_price: null,
        stop_loss: null,
        take_profit: null,
        position_size_units: null,
        position_notional_usd: null,
        entry_fee_usd: null,
        exit_fee_usd: null,
        total_fees_usd: null,
        liquidation_price: null,
        net_profit_usd: null,
        roi_pct: null,
    };
}

/** Per-leg fee estimate: commission% × notional for each leg. */
export function computeFeeLegs(notionalUsd: number, commissionPct: number): { entryFee: number; exitFee: number } {
    if (!isFinite(notionalUsd) || notionalUsd <= 0) return { entryFee: 0, exitFee: 0 };
    const rate = isFinite(commissionPct) ? Math.max(0, commissionPct) / 100 : 0;
    return {
        entryFee: notionalUsd * rate,
        exitFee: notionalUsd * rate,
    };
}

/** ROI on allocated margin: net_pnl / margin_required × 100. */
export function computeRoiPct(netPnl: number, marginRequired: number): number | null {
    if (!isFinite(netPnl) || !isFinite(marginRequired) || marginRequired <= 0) return null;
    return (netPnl / marginRequired) * 100;
}

/**
 * Build a configured `ProjectionState` from a backend `RiskCalculation`.
 * All math is client-side derivation over the endpoint's existing
 * response — no new server math.
 */
export function buildProjection(
    setup: ProjectionSetup,
    capital: number,
    leverage: number,
    commissionPct: number,
    calc: RiskCalculation,
): ProjectionState {
    const notional = parseFloat(calc.position_notional) || 0;
    const netPnl = parseFloat(calc.net_pnl) || 0;
    const margin = parseFloat(calc.margin_required) || 0;
    const fees = computeFeeLegs(notional, commissionPct);
    const netPnlNum = parseFloat(calc.net_pnl);
    return {
        configured: true,
        capital,
        leverage,
        direction: setup.direction,
        entry_price: setup.entry,
        stop_loss: setup.stopLoss,
        take_profit: setup.takeProfit,
        position_size_units: parseFloat(calc.position_size_units) || null,
        position_notional_usd: notional > 0 ? notional : null,
        entry_fee_usd: fees.entryFee > 0 ? fees.entryFee : null,
        exit_fee_usd: fees.exitFee > 0 ? fees.exitFee : null,
        total_fees_usd: parseFloat(calc.total_fees) || null,
        liquidation_price: parseFloat(calc.liquidation_price) || null,
        net_profit_usd: isFinite(netPnlNum) ? netPnlNum : null,
        roi_pct: isFinite(netPnlNum) ? computeRoiPct(netPnlNum, margin) : null,
    };
}
