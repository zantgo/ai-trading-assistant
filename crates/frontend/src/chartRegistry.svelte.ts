import type { IChartApi } from 'lightweight-charts';

let charts: IChartApi[] = [];

export function registerChart(chart: IChartApi) {
    charts.push(chart);
}

export function unregisterChart(chart: IChartApi) {
    charts = charts.filter(c => c !== chart);
}
