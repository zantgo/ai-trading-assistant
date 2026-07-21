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

export function downloadCanvasAsPng(canvas: HTMLCanvasElement, filename: string): void {
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
}

/// Compose a vertical stack of Lightweight Charts canvases into a single
/// PNG image. Each chart canvas is taken via `chart.takeScreenshot()` and
/// pasted onto an opaque background; chart-type labels are rendered above
/// each canvas to disambiguate the panes when the column is exported.
export function composeChartScreenshots(
    entries: { label: string; chart: IChartApi }[],
    filename: string,
    bgColor = '#0a0a0f',
    labelColor = '#64ffda',
    labelBg = 'rgba(19, 23, 34, 0.95)',
): void {
    const captured: { label: string; canvas: HTMLCanvasElement }[] = [];
    for (const entry of entries) {
        try {
            const canvas = entry.chart.takeScreenshot();
            if (canvas) captured.push({ label: entry.label, canvas });
        } catch (_) {}
    }
    if (captured.length === 0) return;

    const labelHeight = 22;
    const gap = 10;
    const totalWidth = Math.max(...captured.map((c) => c.canvas.width), 800);
    const totalHeight = captured.reduce((acc, c) => acc + c.canvas.height + labelHeight + gap, 0);

    const composite = document.createElement('canvas');
    composite.width = totalWidth;
    composite.height = totalHeight;
    const ctx = composite.getContext('2d');
    if (!ctx) return;

    ctx.fillStyle = bgColor;
    ctx.fillRect(0, 0, totalWidth, totalHeight);

    let y = 0;
    for (const entry of captured) {
        ctx.fillStyle = labelBg;
        ctx.fillRect(0, y, totalWidth, labelHeight);
        ctx.fillStyle = labelColor;
        ctx.font = 'bold 12px "Courier New", monospace';
        ctx.textBaseline = 'middle';
        ctx.fillText(entry.label.toUpperCase(), 12, y + labelHeight / 2);
        ctx.fillStyle = bgColor;
        ctx.fillRect(0, y + labelHeight, totalWidth, gap);
        ctx.drawImage(entry.canvas, 0, y + labelHeight + gap);
        y += entry.canvas.height + labelHeight + gap;
    }

    downloadCanvasAsPng(composite, filename);
}
