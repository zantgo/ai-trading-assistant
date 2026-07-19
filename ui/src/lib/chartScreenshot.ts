import type { IChartApi } from 'lightweight-charts';

export function takeChartScreenshot(chart: IChartApi, filename: string): void {
    try {
        const canvas = chart.takeScreenshot();
        if (!canvas) return;
        canvas.toBlob((blob) => {
            if (!blob) return;
            const url = URL.createObjectURL(blob);
            const a = document.createElement('a');
            a.href = url;
            a.download = `${filename}-${Date.now()}.png`;
            document.body.appendChild(a);
            a.click();
            document.body.removeChild(a);
            URL.revokeObjectURL(url);
        }, 'image/png');
    } catch (_) {}
}
