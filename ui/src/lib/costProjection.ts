// costProjection — pure fee/funding projection math for the Profile
// "Fees, Leverage & Cost Projection" tab. Round-trip taker fees plus the
// 8h funding drag a perpetual-futures holder pays over the expected hold.

export interface CostProjectionParams {
    /** Position capital in $ (what-if). */
    capital: number;
    /** Notional multiplier (cross leverage). */
    leverage: number;
    /** Taker fee in % (e.g. 0.06 = 0.06%). */
    takerFeePct: number;
    /** Funding rate per 8h period in % (e.g. 0.01 = 0.01%). */
    fundingRatePct: number;
    /** Number of 8h funding periods expected in the hold. */
    holdPeriods: number;
}

export interface CostProjectionResult {
    notional: number;
    roundTripFees: number;
    fundingDrag: number;
    totalCost: number;
    minProfitPct: number;
}

export function costProjection(p: CostProjectionParams): CostProjectionResult {
    const notional = p.capital * p.leverage;
    const roundTripFees = (p.takerFeePct / 100) * notional * 2;
    const fundingDrag = (p.fundingRatePct / 100) * notional * p.holdPeriods;
    const totalCost = roundTripFees + fundingDrag;
    const minProfitPct = p.capital > 0 ? (totalCost / p.capital) * 100 : 0;
    return { notional, roundTripFees, fundingDrag, totalCost, minProfitPct };
}
