export interface TradeMarkerInput {
    direction: string;
    entry_timestamp: number;
    exit_timestamp?: number;
    symbol: string;
}

export interface ChartMarker {
    time: number;
    position: 'aboveBar' | 'belowBar';
    color: string;
    shape: 'arrowUp' | 'arrowDown';
    text: string;
}

function alignToCandle(ms: number, barDurationSec: number): number {
    const sec = Math.floor(ms / 1000);
    return Math.floor(sec / barDurationSec) * barDurationSec;
}

export function tradeToMarkers(
    trade: TradeMarkerInput,
    barDurationSec: number,
    currentSymbol: string
): ChartMarker[] {
    const tSymbol = String(trade.symbol);
    // Match either the exact symbol or the base token (quote-agnostic), so a
    // trade keyed "BTC-USDC"/"BTC-USDT" still maps to the "BTC" chart.
    const tBase = tSymbol.split('-')[0];
    const curBase = currentSymbol.split('-')[0];
    if (tSymbol !== currentSymbol && tBase !== curBase) {
        return [];
    }

    if (!trade.entry_timestamp || trade.entry_timestamp <= 0) {
        return [];
    }

    const markers: ChartMarker[] = [];
    const entryTime = alignToCandle(trade.entry_timestamp, barDurationSec);

    if (trade.direction === 'LONG') {
        markers.push({
            time: entryTime,
            position: 'belowBar',
            color: '#26a69a',
            shape: 'arrowUp',
            text: 'Open Long',
        });
    } else if (trade.direction === 'SHORT') {
        markers.push({
            time: entryTime,
            position: 'aboveBar',
            color: '#ef5350',
            shape: 'arrowDown',
            text: 'Open Short',
        });
    }

    if (trade.exit_timestamp != null && trade.exit_timestamp > 0) {
        const exitTime = alignToCandle(trade.exit_timestamp, barDurationSec);

        if (trade.direction === 'LONG') {
            markers.push({
                time: exitTime,
                position: 'aboveBar',
                color: '#ef5350',
                shape: 'arrowDown',
                text: 'Close Long',
            });
        } else if (trade.direction === 'SHORT') {
            markers.push({
                time: exitTime,
                position: 'belowBar',
                color: '#26a69a',
                shape: 'arrowUp',
                text: 'Close Short',
            });
        }
    }

    return markers;
}

/**
 * Fetch paper trades + active position and build a deduped, time-sorted set of
 * chart markers for a pair. Returns [] on error.
 */
export async function buildTradeMarkers(
    pairKey: string,
    symbol: string,
    barDurationSec: number,
): Promise<ChartMarker[]> {
    const markersList: ChartMarker[] = [];
    try {
        const perfRes = await fetch(`/api/paper/performance?symbol=${encodeURIComponent(pairKey)}`);
        if (perfRes.ok) {
            const data = await perfRes.json();
            for (const t of data.trades || []) {
                markersList.push(...tradeToMarkers(t, barDurationSec, symbol));
            }
        }
        const statusRes = await fetch(`/api/paper/status?symbol=${encodeURIComponent(pairKey)}`);
        if (statusRes.ok) {
            const statusData = await statusRes.json();
            if (statusData.active_position) {
                markersList.push(...tradeToMarkers(statusData.active_position, barDurationSec, symbol));
            }
        }
    } catch (err) {
        console.error('Error loading trade markers:', err);
    }
    markersList.sort((a, b) => (a.time as number) - (b.time as number));
    const seen = new Set<string>();
    const unique: ChartMarker[] = [];
    for (const m of markersList) {
        const key = `${m.time}-${m.shape}`;
        if (!seen.has(key)) { seen.add(key); unique.push(m); }
    }
    return unique;
}
