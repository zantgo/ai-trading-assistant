import type { IChartApi } from 'lightweight-charts';

let charts: IChartApi[] = [];

let isSyncing = false;

export function registerChart(chart: IChartApi) {
    charts.push(chart);

    chart.timeScale().subscribeVisibleLogicalRangeChange((range) => {
        if (isSyncing || !range) return;
        isSyncing = true;
        charts.forEach((other) => {
            if (other !== chart) {
                other.timeScale().setVisibleLogicalRange(range);
            }
        });
        isSyncing = false;
    });
}

export function unregisterChart(chart: IChartApi) {
    charts = charts.filter(c => c !== chart);
}
