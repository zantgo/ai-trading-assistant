// @vitest-environment node
import { describe, expect, it } from 'vitest';
import { vwapPickKey, type VwapPick } from './vwapAnchor';

describe('vwapPickKey — auto-adapt VWAP anchor to TF duration', () => {
    // Tier-table: each row documents the boundary at which the picker
    // upgrades to the next anchor. The exclusive upper bound of one tier is
    // the inclusive lower bound of the next.
    const cases: Array<{ secs: number; expected: VwapPick; reason: string }> = [
        // Below 1 h: daily vwap.
        { secs: 1,    expected: { arrayKey: 'vwap',          iSubKey: 'vwap'    }, reason: '1 s bar → daily' },
        { secs: 15,   expected: { arrayKey: 'vwap',          iSubKey: 'vwap'    }, reason: '15 s bar (sub-minute) → daily' },
        { secs: 30,   expected: { arrayKey: 'vwap',          iSubKey: 'vwap'    }, reason: '30 s bar (sub-minute) → daily' },
        { secs: 60,   expected: { arrayKey: 'vwap',          iSubKey: 'vwap'    }, reason: '1 m bar → daily' },
        { secs: 300,  expected: { arrayKey: 'vwap',          iSubKey: 'vwap'    }, reason: '5 m bar → daily' },
        { secs: 900,  expected: { arrayKey: 'vwap',          iSubKey: 'vwap'    }, reason: '15 m bar → daily' },
        { secs: 1800, expected: { arrayKey: 'vwap',          iSubKey: 'vwap'    }, reason: '30 m bar → daily (still < 1 h)' },

        // 1 h ≤ tf < 12 h: weekly anchored vwap (1 h boundary is inclusive).
        { secs: 3599, expected: { arrayKey: 'vwap',           iSubKey: 'vwap'    }, reason: 'just below 1 h → still daily' },
        { secs: 3600, expected: { arrayKey: 'avwap_weekly',  iSubKey: 'weekly'  }, reason: 'exactly 1 h → weekly (≥ 1 h)' },
        { secs: 3601, expected: { arrayKey: 'avwap_weekly',  iSubKey: 'weekly'  }, reason: 'just above 1 h → weekly' },
        { secs: 7200, expected: { arrayKey: 'avwap_weekly',  iSubKey: 'weekly'  }, reason: '2 h bar → weekly' },
        { secs: 14400, expected: { arrayKey: 'avwap_weekly', iSubKey: 'weekly' }, reason: '4 h bar → weekly' },
        { secs: 28800, expected: { arrayKey: 'avwap_weekly', iSubKey: 'weekly' }, reason: '8 h bar → weekly' },
        { secs: 43199, expected: { arrayKey: 'avwap_weekly', iSubKey: 'weekly' }, reason: 'just below 12 h → still weekly' },

        // tf ≥ 12 h: monthly anchored vwap.
        { secs: 43200, expected: { arrayKey: 'avwap_monthly', iSubKey: 'monthly' }, reason: 'exactly 12 h → monthly (≥ 12 h)' },
        { secs: 86400, expected: { arrayKey: 'avwap_monthly', iSubKey: 'monthly' }, reason: '1 d bar → monthly' },
        { secs: 604800, expected: { arrayKey: 'avwap_monthly', iSubKey: 'monthly' }, reason: '1 w bar → monthly' },
    ];

    for (const { secs, expected, reason } of cases) {
        it(`${reason} (${secs}s)`, () => {
            expect(vwapPickKey(secs)).toEqual(expected);
        });
    }

    it('returns a stable object identity within the same tier', () => {
        // Two calls in the same tier can return new objects but never
        // escalate tiers spontaneously.
        const a = vwapPickKey(1800);
        const b = vwapPickKey(1800);
        expect(a.arrayKey).toBe(b.arrayKey);
        expect(a.iSubKey).toBe(b.iSubKey);
    });

    it('never returns a swing anchor automatically', () => {
        // Swing VWAP only resets on pivot breakpoints, so it can sit at one
        // value for weeks — it's a manual toggle for power users, not part
        // of the auto-pick.
        for (const secs of [1, 60, 900, 1800, 3600, 7200, 14400, 43200, 86400]) {
            expect(vwapPickKey(secs).iSubKey).not.toBe('swing');
        }
    });
});
