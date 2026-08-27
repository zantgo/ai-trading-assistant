// engineExport — envelope contract test: every engine-tab export is valid
// JSON with the canonical schema, the tab identity and the visible data.
import { describe, expect, it } from 'vitest';
import { buildEngineExport } from './engineExport';

describe('buildEngineExport envelope (v7.3)', () => {
    it('wraps tab data in the canonical schema', () => {
        const json = buildEngineExport('portfolio', 'exposure', 'observe', {
            gross_exposure: '1234.00',
            limits: { max_single_pair_exposure_pct: 20 },
        });
        const parsed = JSON.parse(json);
        expect(parsed.schema).toBe('engine-tab-export/v1');
        expect(parsed.engine).toBe('portfolio');
        expect(parsed.tab).toBe('exposure');
        expect(parsed.mode).toBe('observe');
        expect(typeof parsed.exported_at).toBe('number');
        expect(parsed.data.gross_exposure).toBe('1234.00');
        expect(parsed.data.limits.max_single_pair_exposure_pct).toBe(20);
    });

    it('supports null mode for mode-agnostic engines (DIE)', () => {
        const parsed = JSON.parse(buildEngineExport('data_infra', 'settings', null, { size: 500 }));
        expect(parsed.mode).toBeNull();
        expect(parsed.data.size).toBe(500);
    });

    it('serializes nested arrays (pipelines, trades) without loss', () => {
        const parsed = JSON.parse(
            buildEngineExport('trade_automation', 'history', 'paper', {
                trades: [{ id: 1, realized_pnl: 12.5 }, { id: 2, realized_pnl: -3.0 }],
            }),
        );
        expect(parsed.data.trades).toHaveLength(2);
        expect(parsed.data.trades[1].realized_pnl).toBe(-3.0);
    });
});
