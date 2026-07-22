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
import type { LiquidationCluster, LiquidationClusterMatrix } from '../types';

const DEBUG_TAG = '[LH]';

function dbg(...args: unknown[]): void {
    if (typeof console !== 'undefined' && (globalThis as any).__LH_DEBUG__) {
        console.log(DEBUG_TAG, ...args);
    }
}

/// Minimum intensity below which a cluster is skipped. Picked so that
/// the color ramp's transparent/translucent low-alpha entries are never
/// drawn (those entries waste a fillRect per cell for a near-invisible
/// result). Volume profile has an analogous `MIN_BAR_PX` guard; this is
/// the heatmap's intensity-domain equivalent.
const MIN_INTENSITY = 0.05;
/// Cell height (px) of each row in the rasterized heatmap. Smaller cells
/// look smoother on zoom-in but cost more fills; 3 px is the sweet spot
/// for the typical chart height.
const CELL_HEIGHT_PX = 3;

function intensityColor(intensity: number): string {
    const i = Math.min(1, Math.max(0, intensity));
    if (i <= 0.01) return 'transparent';
    if (i < 0.15) return `rgba(0, 20, 80, ${i * 3})`;
    if (i < 0.30) return `rgba(0, 50, 180, ${i * 1.8})`;
    if (i < 0.50) return `rgba(0, 180, 255, ${i * 1.2})`;
    if (i < 0.70) return `rgba(0, 220, 80, ${i * 0.9})`;
    if (i < 0.85) return `rgba(240, 240, 0, ${i * 0.7})`;
    if (i < 0.95) return `rgba(255, 150, 0, 0.55)`;
    return `rgba(255, 40, 0, 0.55)`;
}

function clusterIntensity(cluster: LiquidationCluster, maxNotional: number): number {
    if (maxNotional <= 0) return 0;
    return Math.min(1, (cluster.notional_usd / maxNotional) * (cluster.magnet_strength / 100));
}

/// Renders an estimated liquidation cluster matrix as horizontal colored
/// bands spanning the full candle pane width. Per-cluster color encodes
/// intensity = (cluster.notional / max notional across all clusters) ×
/// (magnet_strength / 100); the cell-by-cell rasterization mirrors
/// TradingView's liquidation-heatmap convention.
///
/// Architecture (mirrors `VolumeProfilePrimitive`):
/// - `setVisible()` is **decoupled** from `updateData()` so that flipping
///   the toggle pill on/off never nulls the cluster — the previous
///   pattern `updateData(visible ? data : null)` raced with the WS push
///   cadence and could leave the heatmap empty for several candle
///   intervals after a toggle.
/// - `updateData()` has a **deferred-dispatch** fallback via
///   `requestAnimationFrame` for the case where the WS delivers cluster
///   data before `attached()` has fired (early boot / Vite HMR). Without
///   this fallback, the redraw request is silently dropped and the
///   heatmap stays empty until the next WS push.
/// - `attached()` **flushes** any cluster that arrived before attach.
export class LiquidationHeatmapPrimitive implements ISeriesPrimitiveBase<SeriesAttachedParameter<Time, 'Candlestick'>> {
    private _chart: IChartApi;
    private _candleSeries: ISeriesApi<'Candlestick'>;
    private _cluster: LiquidationClusterMatrix | null = null;
    private _requestUpdate?: () => void;
    private _visible: boolean = false;

    constructor(chart: IChartApi, candleSeries: ISeriesApi<'Candlestick'>) {
        this._chart = chart;
        this._candleSeries = candleSeries;
    }

    /// Store the latest cluster. Decoupled from visibility — callers
    /// must invoke `setVisible()` independently if they want to hide
    /// the overlay while preserving the data (toggle pill behavior).
    updateData(cluster: LiquidationClusterMatrix | null | undefined) {
        this._cluster = cluster ?? null;
        if (this._requestUpdate) {
            this._requestUpdate();
            dbg('updateData: short=', this._cluster?.short_clusters.length ?? 0,
                'long=', this._cluster?.long_clusters.length ?? 0,
                'visible=', this._visible);
        } else {
            dbg('updateData: queued (no _requestUpdate yet) clusters=',
                (this._cluster?.short_clusters.length ?? 0) + (this._cluster?.long_clusters.length ?? 0));
            // Defer one rAF and try again — the chart's attached() callback
            // will set _requestUpdate on its next render tick.
            requestAnimationFrame(() => {
                if (this._requestUpdate) {
                    this._requestUpdate();
                    dbg('updateData: deferred dispatch');
                }
            });
        }
    }

    /// Toggle whether the heatmap should be drawn. Independent of
    /// `updateData()` so flipping the pill off then back on does not
    /// race the WS push cadence. Mirrors `VolumeProfilePrimitive.setVisible`.
    setVisible(visible: boolean): void {
        const next = !!visible;
        if (next === this._visible) return;
        this._visible = next;
        if (this._requestUpdate) {
            this._requestUpdate();
        } else {
            requestAnimationFrame(() => {
                if (this._requestUpdate) this._requestUpdate();
            });
        }
        dbg('setVisible(', next, ') hasCluster=', this._cluster != null);
    }

    attached(param: SeriesAttachedParameter<Time, 'Candlestick'>): void {
        this._requestUpdate = param.requestUpdate;
        // If we received cluster data before attached() fired, request a
        // redraw now so the freshly-attached primitive renders the queued
        // matrix. Without this, the heatmap stays empty until the next WS
        // push (which may be minutes away for low-TF slots).
        if (this._cluster) {
            this._requestUpdate();
            dbg('attached: flushed queued cluster');
        }
    }

    detached(): void {
        this._requestUpdate = undefined;
    }

    paneViews(): readonly IPrimitivePaneView[] {
        const self = this;
        return [{
            renderer(): IPrimitivePaneRenderer | null {
                if (!self._visible) return null;
                if (!self._cluster) return null;
                // lightweight-charts' priceToCoordinate throws "Value is null"
                // on an empty series. Bail early so we don't fall into that path.
                try {
                    if (!self._chart.timeScale().getVisibleLogicalRange()) return null;
                } catch (_) {
                    return null;
                }
                return {
                    draw(target: CanvasRenderingTarget2D) {
                        self._renderGrid(target);
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

    private _renderGrid(target: CanvasRenderingTarget2D) {
        const cl = this._cluster;
        if (!cl) return;

        const allClusters = [...(cl.short_clusters ?? []), ...(cl.long_clusters ?? [])];
        if (allClusters.length === 0) return;

        const maxNotional = Math.max(...allClusters.map(c => c.notional_usd || 0), 1);

        target.useMediaCoordinateSpace(({ context: ctx, mediaSize: { width, height } }) => {
            if (width <= 0 || height <= 0) return;

            const topY = 0;
            const bottomY = height;
            let maxIntensity = 0;
            let drawn = 0;
            let skippedIntensity = 0;
            let skippedCoordinate = 0;

            for (const cluster of allClusters) {
                const intensity = clusterIntensity(cluster, maxNotional);
                if (intensity > maxIntensity) maxIntensity = intensity;
                if (intensity < MIN_INTENSITY) { skippedIntensity++; continue; }

                const priceLow = Math.min(cluster.price_low, cluster.price_high);
                const priceHigh = Math.max(cluster.price_low, cluster.price_high);
                if (priceLow <= 0 || priceHigh <= 0) continue;

                const yHigh = this._safePriceToCoordinate(priceHigh);
                const yLow = this._safePriceToCoordinate(priceLow);
                if (yHigh === null || yLow === null) { skippedCoordinate++; continue; }

                const startY = Math.min(yHigh, yLow);
                const endY = Math.max(yHigh, yLow);

                let ry = Math.floor(startY / CELL_HEIGHT_PX) * CELL_HEIGHT_PX;
                while (ry < endY && ry < bottomY) {
                    if (ry >= topY) {
                        ctx.fillStyle = intensityColor(intensity);
                        ctx.fillRect(0, ry, width, CELL_HEIGHT_PX);
                    }
                    ry += CELL_HEIGHT_PX;
                }
                drawn++;
            }

            dbg('_rendered clusters=', drawn, '/', allClusters.length,
                'maxIntensity=', maxIntensity.toFixed(3),
                'skippedIntensity=', skippedIntensity,
                'skippedCoordinate=', skippedCoordinate);
        });
    }
}

export function attachHeatmap(chart: IChartApi, candleSeries: ISeriesApi<'Candlestick'>): LiquidationHeatmapPrimitive {
    const heatmap = new LiquidationHeatmapPrimitive(chart, candleSeries);
    candleSeries.attachPrimitive(heatmap);
    return heatmap;
}
