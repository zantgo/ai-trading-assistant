import type { IChartApi } from 'lightweight-charts';

interface ChartEntry {
    chart: IChartApi;
    container: HTMLElement;
}

let entries: ChartEntry[] = [];

export function registerChart(chart: IChartApi, container: HTMLElement) {
    entries.push({ chart, container });
}

export function unregisterChart(chart: IChartApi) {
    entries = entries.filter((e) => e.chart !== chart);
}

/// Returns every registered chart whose container is contained within the
/// supplied DOM root, in DOM order. Used to compose screenshots of a
/// fullscreen timeframe column that contains multiple pane charts.
export function chartsWithin(root: HTMLElement): ChartEntry[] {
    return entries.filter((e) => root.contains(e.container));
}
