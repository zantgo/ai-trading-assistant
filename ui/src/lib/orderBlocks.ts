// Selective SMC Order Block (OB) primitive. An OB is the candle that
// preceded a Break of Structure and is annotated as either bullish
// (last down-closed candle before a rally) or bearish (last up-closed
// candle before a drop). The renderer draws a translucent band between
// ob_*_high and ob_*_low. New zones are appended on every snapshot
// where state_label indicates ACTIVE/TEST for a side; zones are capped
// at 30 with FIFO eviction.

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

interface ObZone {
    startTime: number;
    high: number;
    low: number;
    direction: 'bullish' | 'bearish';
    state: 'ACTIVE' | 'TEST';
}

const MAX_ZONES = 30;
const BULL_FILL = 'rgba(141, 110, 99, 0.22)';
const BULL_BORDER = 'rgba(215, 204, 200, 0.65)';
const BEAR_FILL = 'rgba(141, 110, 99, 0.22)';
const BEAR_BORDER = 'rgba(215, 204, 200, 0.65)';

export class OrderBlocksPrimitive implements ISeriesPrimitiveBase<SeriesAttachedParameter<Time, 'Candlestick'>> {
    private _chart: IChartApi;
    private _candleSeries: ISeriesApi<'Candlestick'>;
    private _zones: ObZone[] = [];
    private _visible: boolean = false;
    private _requestUpdate?: () => void;
    private _lastSeenKey: string = '';

    constructor(chart: IChartApi, candleSeries: ISeriesApi<'Candlestick'>) {
        this._chart = chart;
        this._candleSeries = candleSeries;
    }

    private _nowSeconds(): number {
        let startTime = Math.floor(Date.now() / 1000);
        try {
            const logical = this._chart.timeScale().getVisibleLogicalRange();
            if (logical) {
                const fromIndex = Math.max(0, Math.floor(logical.from));
                const ts = this._chart.timeScale().coordinateToTime(fromIndex as any) as number | undefined;
                if (typeof ts === 'number') startTime = ts;
            }
        } catch (_) { /* ignore */ }
        return startTime;
    }

    updateData(dto: IndicatorDto | null | undefined) {
        if (!dto) return;
        const v = dto.values ?? {};
        const bullishH = (v['ob_bullish_high'] ?? null) as number | null;
        const bullishL = (v['ob_bullish_low'] ?? null) as number | null;
        const bearishH = (v['ob_bearish_high'] ?? null) as number | null;
        const bearishL = (v['ob_bearish_low'] ?? null) as number | null;
        const label = (dto.state_label ?? '').toUpperCase();

        const candidates: ObZone[] = [];
        if (label.includes('BULLISH') && bullishH != null && bullishL != null && bullishH > 0 && bullishL > 0) {
            const state: 'ACTIVE' | 'TEST' = label.includes('TEST') ? 'TEST' : 'ACTIVE';
            candidates.push({
                startTime: this._nowSeconds(),
                high: Math.max(bullishH, bullishL),
                low: Math.min(bullishH, bullishL),
                direction: 'bullish',
                state,
            });
        }
        if (label.includes('BEARISH') && bearishH != null && bearishL != null && bearishH > 0 && bearishL > 0) {
            const state: 'ACTIVE' | 'TEST' = label.includes('TEST') ? 'TEST' : 'ACTIVE';
            candidates.push({
                startTime: this._nowSeconds(),
                high: Math.max(bearishH, bearishL),
                low: Math.min(bearishH, bearishL),
                direction: 'bearish',
                state,
            });
        }

        if (candidates.length === 0) {
            return;
        }

        const dedupeKey = candidates
            .map((c) => `${c.direction}-${c.high}-${c.low}-${c.state}`)
            .join('|');

        if (dedupeKey !== this._lastSeenKey) {
            for (const z of candidates) {
                this._zones.push(z);
                while (this._zones.length > MAX_ZONES) this._zones.shift();
            }
            this._lastSeenKey = dedupeKey;
        }

        if (this._requestUpdate) this._requestUpdate();
    }

    setVisible(visible: boolean) {
        const next = !!visible;
        if (next === this._visible) return;
        this._visible = next;
        if (this._requestUpdate) this._requestUpdate();
    }

    clear() {
        this._zones = [];
        this._lastSeenKey = '';
        if (this._requestUpdate) this._requestUpdate();
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
                const yHigh = this._safePriceToCoordinate(z.high);
                const yLow = this._safePriceToCoordinate(z.low);
                if (yHigh == null || yLow == null) continue;
                const top = Math.min(yHigh, yLow);
                const bot = Math.max(yHigh, yLow);
                ctx.fillStyle = z.direction === 'bullish' ? BULL_FILL : BEAR_FILL;
                ctx.fillRect(0, top, width, bot - top);
                ctx.strokeStyle = z.direction === 'bullish' ? BULL_BORDER : BEAR_BORDER;
                ctx.lineWidth = z.state === 'TEST' ? 1 : 2;
                if (z.state === 'TEST') {
                    ctx.setLineDash([3, 3]);
                } else {
                    ctx.setLineDash([]);
                }
                ctx.beginPath();
                ctx.moveTo(0, top);
                ctx.lineTo(width, top);
                ctx.moveTo(0, bot);
                ctx.lineTo(width, bot);
                ctx.stroke();
                ctx.setLineDash([]);
            }
            if (zones.length > 0) {
                const bull = zones.filter((z) => z.direction === 'bullish').length;
                const bear = zones.filter((z) => z.direction === 'bearish').length;
                ctx.fillStyle = 'rgba(215, 204, 200, 0.85)';
                ctx.font = '8px "Courier New", monospace';
                ctx.textAlign = 'right';
                ctx.textBaseline = 'top';
                ctx.fillText(`OB × ${zones.length}  B${bull}/R${bear}`, width - 6, 18);
            }
        });
    }
}

export function attachOrderBlocks(chart: IChartApi, candleSeries: ISeriesApi<'Candlestick'>): OrderBlocksPrimitive {
    const p = new OrderBlocksPrimitive(chart, candleSeries);
    candleSeries.attachPrimitive(p);
    return p;
}
