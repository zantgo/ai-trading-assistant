import type { OpportunityMatrix } from '../types';

export interface DirectionalBars {
  bullish: number;
  bearish: number;
  hold: number;
}

export function computeOpportunityBars(opp: OpportunityMatrix | null): DirectionalBars {
  if (!opp) return { bullish: 0, bearish: 0, hold: 100 };

  const longRR = Math.max(0, opp.long_expected_rr_internal ?? 0);
  const shortRR = Math.max(0, opp.short_expected_rr_internal ?? 0);
  const score = Math.max(0, Math.min(100, opp.opportunity_score ?? 0));

  if (longRR === 0 && shortRR === 0) {
    return { bullish: 0, bearish: 0, hold: 100 };
  }

  const wLong = Math.exp(longRR * 3);
  const wShort = Math.exp(shortRR * 3);
  const wHold = Math.exp(0.25);
  const totalW = wLong + wShort + wHold;

  let bullish = (wLong / totalW) * 100;
  let bearish = (wShort / totalW) * 100;
  let hold = (wHold / totalW) * 100;

  const dirTotal = bullish + bearish;
  if (dirTotal > score) {
    const scale = score / dirTotal;
    bullish *= scale;
    bearish *= scale;
    hold = 100 - bullish - bearish;
  }

  const rounded = {
    bullish: Math.round(bullish),
    bearish: Math.round(bearish),
    hold: Math.round(hold),
  };
  const sum = rounded.bullish + rounded.bearish + rounded.hold;
  if (sum !== 100) {
    rounded.hold += 100 - sum;
  }

  return rounded;
}
