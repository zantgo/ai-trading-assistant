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

export class LiquidationHeatmapPrimitive implements ISeriesPrimitiveBase<SeriesAttachedParameter<Time, 'Candlestick'>> {
    private _chart: IChartApi;
    private _candleSeries: ISeriesApi<'Candlestick'>;
    private _cluster: LiquidationClusterMatrix | null = null;
    private _requestUpdate?: () => void;

    constructor(chart: IChartApi, candleSeries: ISeriesApi<'Candlestick'>) {
        this._chart = chart;
        this._candleSeries = candleSeries;
    }

    updateData(cluster: LiquidationClusterMatrix | null | undefined) {
        this._cluster = cluster ?? null;
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

        const visibleRange = this._chart.timeScale().getVisibleRange();
        if (!visibleRange) return;

        target.useMediaCoordinateSpace(({ context: ctx, mediaSize: { width, height } }) => {
            if (width <= 0 || height <= 0) return;

            const topY = 0;
            const bottomY = height;
            const cellHeight = 3;

            for (const cluster of allClusters) {
                const intensity = clusterIntensity(cluster, maxNotional);
                if (intensity <= 0.01) continue;

                const priceLow = Math.min(cluster.price_low, cluster.price_high);
                const priceHigh = Math.max(cluster.price_low, cluster.price_high);
                if (priceLow <= 0 || priceHigh <= 0) continue;

                const yHigh = this._safePriceToCoordinate(priceHigh);
                const yLow = this._safePriceToCoordinate(priceLow);
                if (yHigh === null || yLow === null) continue;

                const startY = Math.min(yHigh, yLow);
                const endY = Math.max(yHigh, yLow);

                let ry = Math.floor(startY / cellHeight) * cellHeight;
                while (ry < endY && ry < bottomY) {
                    if (ry >= topY) {
                        ctx.fillStyle = intensityColor(intensity);
                        ctx.fillRect(0, ry, width, cellHeight);
                    }
                    ry += cellHeight;
                }
            }
        });
    }
}

export function attachHeatmap(chart: IChartApi, candleSeries: ISeriesApi<'Candlestick'>): LiquidationHeatmapPrimitive {
    const heatmap = new LiquidationHeatmapPrimitive(chart, candleSeries);
    candleSeries.attachPrimitive(heatmap);
    return heatmap;
}
