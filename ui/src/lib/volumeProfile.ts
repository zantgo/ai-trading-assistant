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
import type { VolumeProfileSnapshot } from '../types';

const POC_COLOR = 'rgba(255, 220, 80, 0.95)';
const POC_BAR_BG = 'rgba(255, 220, 80, 0.10)';
const VA_COLOR = 'rgba(120, 220, 255, 0.55)';
const NON_VA_COLOR = 'rgba(160, 160, 160, 0.35)';
const BUY_COLOR = 'rgba(38, 166, 154, 0.85)';
const SELL_COLOR = 'rgba(239, 83, 80, 0.85)';

/**
 * Renders a volume profile on the right edge of the chart as a horizontal
 * histogram with stacked buy/sell split per bin. Anchored to the candle
 * series price scale via `priceToCoordinate()`.
 *
 * Style follows TradingView's default Volume Profile:
 * - Bars are drawn over the rightmost ~12% of the visible chart area.
 * - Per bin: top half is buy volume (green), bottom half is sell volume (red).
 * - Bar length = `bin.volume / max(bins.volume)` * usable_width.
 * - POC bin: bright yellow border, thicker bar, light yellow fill.
 * - Value-area bins: cyan tint; bins outside value area: grey tint.
 */
export class VolumeProfilePrimitive implements ISeriesPrimitiveBase<SeriesAttachedParameter<Time, 'Candlestick'>> {
    private _chart: IChartApi;
    private _candleSeries: ISeriesApi<'Candlestick'>;
    private _snapshot: VolumeProfileSnapshot | null = null;
    private _requestUpdate?: () => void;
    /// Fraction of the right edge reserved for the histogram.
    private static readonly RIGHT_EDGE_FRACTION = 0.12;
    /// Minimum bar width in pixels (for legibility).
    private static readonly MIN_BAR_PX = 2;
    /// Minimum bin thickness in pixels (collapse to single color if smaller).
    private static readonly MIN_BIN_THICKNESS_PX = 4;

    constructor(chart: IChartApi, candleSeries: ISeriesApi<'Candlestick'>) {
        this._chart = chart;
        this._candleSeries = candleSeries;
    }

    updateData(snapshot: VolumeProfileSnapshot | null | undefined) {
        this._snapshot = snapshot ?? null;
        this._requestUpdate?.();
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
                if (!self._snapshot || self._snapshot.bins.length === 0) return null;
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
        const snap = this._snapshot;
        if (!snap || snap.bins.length === 0) return;

        target.useMediaCoordinateSpace(({ context: ctx, mediaSize: { width, height } }) => {
            if (width <= 0 || height <= 0) return;

            const usableWidth = Math.max(VolumeProfilePrimitive.MIN_BAR_PX, width * VolumeProfilePrimitive.RIGHT_EDGE_FRACTION);
            const rightX = width;
            const leftX = Math.floor(rightX - usableWidth);

            // Find max volume to normalize bar lengths.
            let maxVol = 0;
            for (const b of snap.bins) {
                if (b.volume > maxVol) maxVol = b.volume;
            }
            if (maxVol <= 0) return;

            // Draw each bin (sorted ascending by price_low).
            for (const bin of snap.bins) {
                const yHigh = this._safePriceToCoordinate(bin.price_high);
                const yLow = this._safePriceToCoordinate(bin.price_low);
                if (yHigh === null || yLow === null) continue;

                const topY = Math.min(yHigh, yLow);
                const botY = Math.max(yHigh, yLow);
                const thickness = botY - topY;

                // Normalize bar length.
                const barLen = Math.max(
                    VolumeProfilePrimitive.MIN_BAR_PX,
                    (bin.volume / maxVol) * usableWidth,
                );
                const barRight = rightX;
                const barLeft = barRight - barLen;

                // Background tint: POC > VA > non-VA.
                let bgFill: string;
                let barBorder: string | null = null;
                let barBorderWidth = 1;
                if (bin.is_poc) {
                    bgFill = POC_BAR_BG;
                    barBorder = POC_COLOR;
                    barBorderWidth = 2;
                } else if (bin.is_value_area) {
                    bgFill = VA_COLOR;
                } else {
                    bgFill = NON_VA_COLOR;
                }

                // Draw background bar fill.
                ctx.fillStyle = bgFill;
                ctx.fillRect(barLeft, topY, barRight - barLeft, thickness);

                // Stacked buy/sell split.
                const total = bin.volume;
                if (total > 0 && thickness >= VolumeProfilePrimitive.MIN_BIN_THICKNESS_PX) {
                    const buyFrac = bin.buy_volume / total;
                    const buyHeight = thickness * buyFrac;
                    const sellHeight = thickness - buyHeight;

                    // Buy: top portion, anchored at topY.
                    if (buyHeight > 0) {
                        ctx.fillStyle = BUY_COLOR;
                        const buyLeft = barRight - barLen * buyFrac * Math.min(1, barLen / usableWidth);
                        ctx.fillRect(buyLeft, topY, barRight - buyLeft, buyHeight);
                    }
                    // Sell: bottom portion, anchored at botY.
                    if (sellHeight > 0) {
                        ctx.fillStyle = SELL_COLOR;
                        const sellLeft = barRight - barLen * (1 - buyFrac) * Math.min(1, barLen / usableWidth);
                        ctx.fillRect(sellLeft, topY + buyHeight, barRight - sellLeft, sellHeight);
                    }
                }

                // POC border (drawn last so it sits on top of the bar).
                if (barBorder) {
                    ctx.strokeStyle = barBorder;
                    ctx.lineWidth = barBorderWidth;
                    ctx.strokeRect(barLeft, topY, barRight - barLeft, thickness);
                }
            }

            // Draw POC label outside the chart if there's room.
            if (snap.poc_price > 0) {
                const yPoc = this._safePriceToCoordinate(snap.poc_price);
                if (yPoc !== null && yPoc >= 0 && yPoc <= height) {
                    ctx.fillStyle = POC_COLOR;
                    ctx.font = '9px "Courier New", monospace';
                    ctx.textAlign = 'right';
                    ctx.textBaseline = 'middle';
                    ctx.fillText('POC', rightX - 4, yPoc);
                }
            }

            // Draw VAH/VAL labels (right-aligned, just outside the bars).
            if (snap.value_area_high > 0 && snap.value_area_high !== snap.value_area_low) {
                const yVah = this._safePriceToCoordinate(snap.value_area_high);
                const yVal = this._safePriceToCoordinate(snap.value_area_low);
                if (yVah !== null) {
                    ctx.fillStyle = VA_COLOR;
                    ctx.font = '8px "Courier New", monospace';
                    ctx.textAlign = 'right';
                    ctx.textBaseline = 'bottom';
                    ctx.fillText('VAH', rightX - 4, yVah - 1);
                }
                if (yVal !== null) {
                    ctx.fillStyle = VA_COLOR;
                    ctx.font = '8px "Courier New", monospace';
                    ctx.textAlign = 'right';
                    ctx.textBaseline = 'top';
                    ctx.fillText('VAL', rightX - 4, yVal + 1);
                }
            }
        });
    }
}

export function attachVolumeProfile(chart: IChartApi, candleSeries: ISeriesApi<'Candlestick'>): VolumeProfilePrimitive {
    const vp = new VolumeProfilePrimitive(chart, candleSeries);
    candleSeries.attachPrimitive(vp);
    return vp;
}