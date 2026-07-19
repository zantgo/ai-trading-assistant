<script lang="ts">
    import styles from './ChartFullscreenOverlay.module.css';

    interface Props {
        open: boolean;
        title: string;
        chart: import('lightweight-charts').IChartApi | null;
        onclose: () => void;
    }

    let { open, title, chart, onclose }: Props = $props();
    let chartDiv = $state<HTMLDivElement | null>(null);

    function handleKeydown(e: KeyboardEvent) {
        if (open && e.key === 'Escape') onclose();
    }

    $effect(() => {
        if (open && chart && chartDiv) {
            requestAnimationFrame(() => {
                const rect = chartDiv!.getBoundingClientRect();
                if (rect.width > 0 && rect.height > 0) {
                    chart!.resize(rect.width, rect.height);
                }
            });
        }
    });

    function takeScreenshot() {
        if (!chart) return;
        try {
            const canvas = chart.takeScreenshot();
            if (!canvas) return;
            canvas.toBlob((blob) => {
                if (!blob) return;
                const url = URL.createObjectURL(blob);
                const a = document.createElement('a');
                a.href = url;
                a.download = `chart-${title.toLowerCase().replace(/\s+/g, '-')}-${Date.now()}.png`;
                document.body.appendChild(a);
                a.click();
                document.body.removeChild(a);
                URL.revokeObjectURL(url);
            }, 'image/png');
        } catch (_) {}
    }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open}
    <div class={styles.backdrop} role="presentation" onclick={onclose}>
        <div class={styles.content} onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}
             role="dialog" aria-modal="true" tabindex="-1">
            <div class={styles.header}>
                <span class={styles.title}>{title}</span>
                <div class={styles.actions}>
                    <button class={styles.screenshotBtn} onclick={takeScreenshot}>Screenshot</button>
                    <button class={styles.closeBtn} onclick={onclose}>✕</button>
                </div>
            </div>
            <div bind:this={chartDiv} class={styles.chartBody}></div>
        </div>
    </div>
{/if}
