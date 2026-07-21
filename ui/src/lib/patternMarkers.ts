// Selective chart-pattern, candlestick-pattern, and OI-Price-Divergence
// markers. Mirrors `smcMarkers.ts` but draws **above-bar** labels for
// bearish patterns and **below-bar** for bullish, with confidence ≥ 0.7
// filter. Wired into `PriceChart.svelte` via three independent toggle
// pills (`PATTERNS`, `CANDLESTICK`, `OI-PRICE DIV`).
//
// Marker labels (priority-ranked):
//   CHART-PATTERN
//     H&S↑ / H&S↓   head & shoulders
//     TT↑ / TT↓    triple top/bottom
//     DB↑ / DB↓    double bottom / top
//     CUPUP / CUPDN cup with handle
//
//   CANDLESTICK
//     MOR↑ / MOR↓  morning / evening star
//     ENG↑ / ENG↓  engulfing
//     HAM↑ / HAM↓  hammer / hanging man
//     DOJI         indecision
//
//   OI-PRICE DIVERGENCE
//     OI↑           price down, OI up (continuation down)
//     OI↓           price up, OI down (continuation up)

import { createSeriesMarkers } from 'lightweight-charts';
import type { ISeriesApi, Time } from 'lightweight-charts';
import type { IndicatorDto } from '../types';

const CONF_THRESHOLD = 0.7;

export interface PatternMarkerController {
    pushPatterns(timeSec: number, dto: IndicatorDto | null | undefined): void;
    pushCandlestick(timeSec: number, dto: IndicatorDto | null | undefined): void;
    pushOiPriceDiv(timeSec: number, dto: IndicatorDto | null | undefined): void;
    clear(): void;
}

interface LwcMarker {
    time: Time;
    position: 'aboveBar' | 'belowBar' | 'inBar';
    color: string;
    shape: 'arrowUp' | 'arrowDown' | 'circle' | 'square';
    text: string;
}

function shouldEmit(direction: 'Bullish' | 'Bearish' | 'Neutral', threshold: number): { bull: boolean; bear: boolean } {
    return {
        bull: direction === 'Bullish',
        bear: direction === 'Bearish',
    };
}

function makeMarker(timeSec: number, position: 'aboveBar' | 'belowBar', color: string, shape: 'arrowUp' | 'arrowDown' | 'circle', text: string): LwcMarker {
    return {
        time: timeSec as Time,
        position,
        color,
        shape,
        text,
    };
}

/// Pattern shape → 2-3 letter label and color.
function patternSpec(label: string, direction: 'Bullish' | 'Bearish' | 'Neutral'): { text: string; color: string; position: 'aboveBar' | 'belowBar'; shape: 'arrowUp' | 'arrowDown' | 'circle' } | null {
    const L = (label || '').toUpperCase();
    if (direction !== 'Bullish' && direction !== 'Bearish') {
        return { text: 'PT', color: '#f59e0b', position: 'aboveBar', shape: 'circle' };
    }
    if (L.includes('HEAD') && L.includes('SHOULDER')) {
        return { text: 'H&S' + (direction === 'Bullish' ? '↑' : '↓'), color: direction === 'Bullish' ? '#10b981' : '#ef4444', position: direction === 'Bullish' ? 'belowBar' : 'aboveBar', shape: direction === 'Bullish' ? 'arrowUp' : 'arrowDown' };
    }
    if (L.includes('TRIPLE_TOP') || L === 'TT') {
        return { text: 'TT↓', color: '#ef4444', position: 'aboveBar', shape: 'arrowDown' };
    }
    if (L.includes('TRIPLE_BOTTOM') || L === 'DBT' || L === 'TB') {
        return { text: 'TB↑', color: '#10b981', position: 'belowBar', shape: 'arrowUp' };
    }
    if (L.includes('DOUBLE_TOP') || L.includes('DOUBLE-TOP') || L === 'DT') {
        return { text: 'DT↓', color: '#ef4444', position: 'aboveBar', shape: 'arrowDown' };
    }
    if (L.includes('DOUBLE_BOTTOM') || L.includes('DOUBLE-BOTTOM') || L === 'DB') {
        return { text: 'DB↑', color: '#10b981', position: 'belowBar', shape: 'arrowUp' };
    }
    if (L.includes('CUP') && L.includes('HANDLE')) {
        return { text: direction === 'Bullish' ? 'CU↑' : 'CU↓', color: direction === 'Bullish' ? '#10b981' : '#ef4444', position: direction === 'Bullish' ? 'belowBar' : 'aboveBar', shape: direction === 'Bullish' ? 'arrowUp' : 'arrowDown' };
    }
    if (L.includes('FLAG') || L.includes('PENNANT')) {
        return { text: direction === 'Bullish' ? 'FL↑' : 'FL↓', color: direction === 'Bullish' ? '#10b981' : '#ef4444', position: direction === 'Bullish' ? 'belowBar' : 'aboveBar', shape: direction === 'Bullish' ? 'arrowUp' : 'arrowDown' };
    }
    return { text: 'PT', color: '#f59e0b', position: 'aboveBar', shape: direction === 'Bullish' ? 'arrowUp' : 'arrowDown' };
}

function candlestickSpec(label: string, direction: 'Bullish' | 'Bearish' | 'Neutral'): { text: string; color: string; position: 'aboveBar' | 'belowBar' | 'inBar'; shape: 'arrowUp' | 'arrowDown' | 'circle' } | null {
    const L = (label || '').toUpperCase();
    if (L.includes('MORNING') || L.includes('MORN')) {
        return { text: 'MOR↑', color: '#10b981', position: 'belowBar', shape: 'arrowUp' };
    }
    if (L.includes('EVENING')) {
        return { text: 'EVE↓', color: '#ef4444', position: 'aboveBar', shape: 'arrowDown' };
    }
    if (L.includes('BULLISH_ENGULFING') || L.includes('ENGULFING_UP') || L === 'ENG↑') {
        return { text: 'ENG↑', color: '#10b981', position: 'belowBar', shape: 'arrowUp' };
    }
    if (L.includes('BEARISH_ENGULFING') || L.includes('ENGULFING_DN') || L === 'ENG↓') {
        return { text: 'ENG↓', color: '#ef4444', position: 'aboveBar', shape: 'arrowDown' };
    }
    if (L.includes('HAMMER') && !L.includes('HANGING')) {
        return { text: 'HAM↑', color: '#10b981', position: 'belowBar', shape: 'arrowUp' };
    }
    if (L.includes('HANGING')) {
        return { text: 'HAM↓', color: '#ef4444', position: 'aboveBar', shape: 'arrowDown' };
    }
    if (L.includes('DOJI')) {
        return { text: 'DOJI', color: '#f59e0b', position: 'inBar', shape: 'circle' };
    }
    if (L.includes('SHOOTING')) {
        return { text: 'SH↓', color: '#ef4444', position: 'aboveBar', shape: 'arrowDown' };
    }
    if (L.includes('INVERTED')) {
        return { text: 'IHAM↑', color: '#10b981', position: 'belowBar', shape: 'arrowUp' };
    }
    if (L.includes('SPINNING')) {
        return { text: 'SPIN', color: '#f59e0b', position: 'inBar', shape: 'circle' };
    }
    if (L.includes('MARUBOZU') || L.includes('MARUBOZU')) {
        return { text: direction === 'Bullish' ? 'MAR↑' : 'MAR↓', color: direction === 'Bullish' ? '#10b981' : '#ef4444', position: direction === 'Bullish' ? 'belowBar' : 'aboveBar', shape: direction === 'Bullish' ? 'arrowUp' : 'arrowDown' };
    }
    if (L.includes('PIERCING')) {
        return { text: 'PRC↑', color: '#10b981', position: 'belowBar', shape: 'arrowUp' };
    }
    if (L.includes('DARK_CLOUD') || L.includes('DARKCLOUD')) {
        return { text: 'DC↓', color: '#ef4444', position: 'aboveBar', shape: 'arrowDown' };
    }
    return { text: 'CD', color: '#f59e0b', position: 'inBar', shape: 'circle' };
}

function oiDivSpec(label: string, direction: 'Bullish' | 'Bearish' | 'Neutral'): { text: string; color: string; position: 'aboveBar' | 'belowBar'; shape: 'arrowUp' | 'arrowDown' | 'circle' } | null {
    const L = (label || '').toUpperCase();
    // OI-Price Divergence: price up + OI down = continuation up (bullish); price down + OI up = continuation down (bearish).
    if (L.includes('BULL') || direction === 'Bullish') {
        return { text: 'OI↑', color: '#10b981', position: 'belowBar', shape: 'arrowUp' };
    }
    if (L.includes('BEAR') || direction === 'Bearish') {
        return { text: 'OI↓', color: '#ef4444', position: 'aboveBar', shape: 'arrowDown' };
    }
    return null;
}

export function createPatternMarkers(
    series: ISeriesApi<'Candlestick'>,
): PatternMarkerController {
    const api = createSeriesMarkers(series, []);
    const markers: LwcMarker[] = [];
    const seen = new Set<string>();
    const MAX = 60;

    function rebuild() {
        markers.sort((a, b) => (a.time as number) - (b.time as number));
        api.setMarkers(markers);
    }

    function ingest(timeSec: number, dto: IndicatorDto | null | undefined, kind: 'patterns' | 'candlestick' | 'oi_price_div') {
        if (!dto) return;
        const sigs = (dto.signals ?? []) as Array<{ kind: string; direction: 'Bullish' | 'Bearish' | 'Neutral'; status: string; label: string; strength: number; confidence?: number }>;
        const dtoConf = dto.confidence;
        for (const sig of sigs) {
            if (typeof dtoConf === 'number' && dtoConf < CONF_THRESHOLD && typeof sig.confidence !== 'number') continue;
            const label = sig.label ?? '';
            let spec: { text: string; color: string; position: 'aboveBar' | 'belowBar' | 'inBar'; shape: 'arrowUp' | 'arrowDown' | 'circle' } | null = null;
            if (kind === 'patterns') {
                spec = patternSpec(label, sig.direction);
            } else if (kind === 'candlestick') {
                spec = candlestickSpec(label, sig.direction);
            } else if (kind === 'oi_price_div') {
                spec = oiDivSpec(label, sig.direction);
            }
            if (!spec) continue;
            const key = `${timeSec}-${kind}-${spec.text}-${spec.position}`;
            if (seen.has(key)) continue;
            seen.add(key);
            markers.push({
                time: timeSec as Time,
                position: spec.position,
                color: spec.color,
                shape: spec.shape as any,
                text: spec.text,
            });
        }
        while (markers.length > MAX) {
            const dropped = markers.shift();
            if (dropped) seen.delete(`${dropped.time as number}-${dropped.text}-${dropped.position}`);
        }
    }

    return {
        pushPatterns(timeSec, dto) { ingest(timeSec, dto, 'patterns'); rebuild(); },
        pushCandlestick(timeSec, dto) { ingest(timeSec, dto, 'candlestick'); rebuild(); },
        pushOiPriceDiv(timeSec, dto) { ingest(timeSec, dto, 'oi_price_div'); rebuild(); },
        clear() {
            markers.length = 0;
            seen.clear();
            api.setMarkers([]);
        },
    };
}
