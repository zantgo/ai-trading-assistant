// Selective SMC Fair Value Gap (FVG) primitive. Each FVG is a 3-candle
// imbalance (low[2] > high[0] for bearish, high[2] < low[0] for bullish);
// the renderer draws a translucent rectangle covering the candle-range of
// the imbalance. New zones are appended on every snapshot where the
// normalized state_label flips to a BULLISH/BEARISH_OPEN; zones are
// capped at 30 to avoid memory blow-up.
//
// Bullish FVG (gap up, price often fills downward) → green tint at bottom.
// Bearish FVG (gap down, price often fills upward) → red tint at top.
//
// zOrder: 'top' so the FVG box paints below candle bodies (it doesn't
// compete with the bullish/bearish body colors but provides visible context).

import type {
    IChartApi,
    ISeriesApi,
    ISeriesPrimitiveBase,
    SeriesAttachedParameter,
    IPrimitivePaneView,
    IPrimitivePaneRenderer,
    Time,
} from 'lightweight-charts';
import type { CanvasRenderingTarget2D } from 'fancy-canvas';
import type { IndicatorDto } from '../types';

interface FvgZone {
    startTime: number;
    top: number;
    bottom: number;
    bullish: boolean;
}

const MAX_ZONES = 30;
const BULL_FILL = 'rgba(38, 166, 154, 0.18)';
const BULL_BORDER = 'rgba(38, 166, 154, 0.55)';
const BEAR_FILL = 'rgba(239, 83, 80, 0.18)';
const BEAR_BORDER = 'rgba(239, 83, 80, 0.55)';

export class FvgZonesPrimitive implements ISeriesPrimitiveBase<SeriesAttachedParameter<Time, 'Candlestick'>> {
    private _chart: IChartApi;
    private _candleSeries: ISeriesApi<'Candlestick'>;
    private _zones: FvgZone[] = [];
    private _visible: boolean = false;
    private _requestUpdate?: () => void;
    private _lastSeen: { top?: number | null; bottom?: number | null; bullish?: boolean } = {};

    constructor(chart: IChartApi, candleSeries: ISeriesApi<'Candlestick'>) {
        this._chart = chart;
        this._candleSeries = candleSeries;
    }

    /// Replace the rolling zone list. Called from `$effect` whenever the
    /// active TF fires a new snapshot carrying an `smc_fvg` entry.
    updateData(dto: IndicatorDto | null | undefined) {
        if (!dto) {
            return;
        }
        const v = dto.values ?? {};
        const topRaw = v['fvg_top'] ?? null;
        const bottomRaw = v['fvg_bottom'] ?? null;
        const bullishRaw = v['fvg_bullish'];
        const top = typeof topRaw === 'number' ? topRaw : null;
        const bottom = typeof bottomRaw === 'number' ? bottomRaw : null;
        const bullish = bullishRaw != null ? bullishRaw >= 0.5 : false;
        const label = dto.state_label ?? '';
        const isOpen = label.includes('BULLISH_OPEN') || label.includes('BEARISH_OPEN');

        if (!isOpen || top == null || bottom == null) {
            // Reset seen so a future FVG re-arms the "new zone" detector.
            this._lastSeen = {};
            this._ping();
            return;
        }

        // De-dupe consecutive snapshots that report the *same* FVG (the same
        // top/bottom/bullish tuple). Only push a new zone when something
        // actually changes.
        const lastTop = this._lastSeen.top ?? null;
        const lastBottom = this._lastSeen.bottom ?? null;
        const lastBull = this._lastSeen.bullish;
        const same = lastTop != null && lastBottom != null
            && Math.abs(lastTop - top) < 1e-8
            && Math.abs(lastBottom - bottom) < 1e-8
            && bullish === lastBull;
        if (!same) {
            // Determine a start time. We don't have a true origin on the wire
            // (only the current candle). Use the chart's last visible time
            // as a reasonable proxy so the rectangle extends from the right.
            let startTime = Math.floor(Date.now() / 1000);
            try {
                const logical = this._chart.timeScale().getVisibleLogicalRange();
                if (logical) {
                    const fromIndex = Math.max(0, Math.floor(logical.from));
                    const ts = this._chart.timeScale().coordinateToTime(fromIndex as any) as number | undefined;
                    if (typeof ts === 'number') startTime = ts;
                }
            } catch (_) { /* ignore */ }
            this._zones.push({
                startTime,
                top: Math.max(top, bottom),
                bottom: Math.min(top, bottom),
                bullish,
            });
            while (this._zones.length > MAX_ZONES) this._zones.shift();
        }
        this._lastSeen = { top, bottom, bullish };
        this._ping();
    }

    private _ping() {
        if (this._requestUpdate) {
            this._requestUpdate();
        } else {
            requestAnimationFrame(() => {
                if (this._requestUpdate) this._requestUpdate();
            });
        }
    }

    setVisible(visible: boolean) {
        const next = !!visible;
        if (next === this._visible) return;
        this._visible = next;
        this._ping();
    }

    clear() {
        this._zones = [];
        this._lastSeen = {};
        this._ping();
    }

    attached(param: SeriesAttachedParameter<Time, 'Candlestick'>): void {
        this._requestUpdate = param.requestUpdate;
    }

    detached(): void {
        this._requestUpdate = undefined;
    }

    paneViews(): readonly IPrimitivePaneView[] {
        const self = this;
        return [{
            renderer(): IPrimitivePaneRenderer | null {
                if (!self._visible) return null;
                if (self._zones.length === 0) return null;
                try {
                    if (!self._chart.timeScale().getVisibleLogicalRange()) return null;
                } catch (_) {
                    return null;
                }
                return {
                    draw(target: CanvasRenderingTarget2D) {
                        self._render(target);
                    },
                };
            },
            zOrder(): 'top' {
                return 'top';
            },
        }];
    }

    private _safePriceToCoordinate(price: number): number | null {
        try {
            return this._candleSeries.priceToCoordinate(price);
        } catch (_) {
            return null;
        }
    }

    private _render(target: CanvasRenderingTarget2D) {
        const zones = this._zones;
        target.useMediaCoordinateSpace(({ context: ctx, mediaSize: { width, height } }) => {
            if (width <= 0 || height <= 0) return;
            for (const z of zones) {
                const yTop = this._safePriceToCoordinate(z.top);
                const yBot = this._safePriceToCoordinate(z.bottom);
                if (yTop == null || yBot == null) continue;
                const top = Math.min(yTop, yBot);
                const bot = Math.max(yTop, yBot);
                const fill = z.bullish ? BULL_FILL : BEAR_FILL;
                const border = z.bullish ? BULL_BORDER : BEAR_BORDER;
                ctx.fillStyle = fill;
                ctx.fillRect(0, top, width, bot - top);
                ctx.strokeStyle = border;
                ctx.lineWidth = 1;
                ctx.beginPath();
                ctx.moveTo(0, top);
                ctx.lineTo(width, top);
                ctx.moveTo(0, bot);
                ctx.lineTo(width, bot);
                ctx.stroke();
            }
            // FVG legend label (drawn once at top-right when zones exist).
            if (zones.length > 0) {
                const last = zones[zones.length - 1];
                ctx.fillStyle = last.bullish ? BULL_BORDER : BEAR_BORDER;
                ctx.font = '8px "Courier New", monospace';
                ctx.textAlign = 'right';
                ctx.textBaseline = 'top';
                ctx.fillText(`FVG × ${zones.length}  ${last.bullish ? '↑' : '↓'}`, width - 6, 4);
            }
        });
    }
}

export function attachFvgZones(chart: IChartApi, candleSeries: ISeriesApi<'Candlestick'>): FvgZonesPrimitive {
    const p = new FvgZonesPrimitive(chart, candleSeries);
    candleSeries.attachPrimitive(p);
    return p;
}
