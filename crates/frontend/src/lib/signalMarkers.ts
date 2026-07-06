// Rolling on-chart signal markers for oscillator panes. Converts an indicator's
// discrete signals (divergence, threshold, breakout, ...) into Lightweight-Charts
// series markers, de-duplicated by (time, kind, direction) and capped.

import { createSeriesMarkers } from 'lightweight-charts';
import type { ISeriesApi, Time } from 'lightweight-charts';
import type { IndicatorSignal } from '../types';

const ABBR: Record<string, string> = {
    Divergence: 'DIV', Breakout: 'BO', Threshold: 'TH', TrendFlip: 'FLIP',
    CompressionRelease: 'SQZ', Crossover: 'X', ZeroLineCross: '0X',
    LevelTest: 'LV', BandTouch: 'BT', VolumeClimax: 'VOL', PatternForming: 'PAT',
};

export interface SignalMarkerController {
    push(timeSec: number, signals: IndicatorSignal[]): void;
}

/**
 * Attach a rolling marker layer to a line series. Only signal `kinds` in the
 * allowlist are drawn (empty = all). Returns a controller whose `push` is called
 * on each new snapshot with that indicator's signals.
 */
export function createSignalMarkers(
    series: ISeriesApi<'Line'>,
    kinds: string[] = [],
): SignalMarkerController {
    const api = createSeriesMarkers(series, []);
    const markers: any[] = [];
    const seen = new Set<string>();
    return {
        push(timeSec: number, signals: IndicatorSignal[]) {
            let changed = false;
            for (const s of signals) {
                if (kinds.length && !kinds.includes(s.kind)) continue;
                const key = `${timeSec}-${s.kind}-${s.direction}`;
                if (seen.has(key)) continue;
                seen.add(key);
                const bull = s.direction === 'Bullish';
                const bear = s.direction === 'Bearish';
                markers.push({
                    time: timeSec as Time,
                    position: bull ? 'belowBar' : 'aboveBar',
                    color: bull ? '#10b981' : bear ? '#ef4444' : '#f59e0b',
                    shape: bull ? 'arrowUp' : bear ? 'arrowDown' : 'circle',
                    text: ABBR[s.kind] ?? s.kind.slice(0, 3),
                });
                changed = true;
            }
            if (changed) {
                if (markers.length > 60) markers.splice(0, markers.length - 60);
                markers.sort((a, b) => (a.time as number) - (b.time as number));
                api.setMarkers(markers);
            }
        },
    };
}
