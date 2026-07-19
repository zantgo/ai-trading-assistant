<script lang="ts">
    interface Props {
        open: boolean;
        title: string;
        chart: import('lightweight-charts').IChartApi | null;
        onclose: () => void;
    }

    let { open, title, chart, onclose }: Props = $props();

    function handleKeydown(e: KeyboardEvent) {
        if (open && e.key === 'Escape') onclose();
    }

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
    <div class="toolbar">
        <span class="title">{title}</span>
        <div class="actions">
            <button class="screenshotBtn" onclick={takeScreenshot}>Screenshot</button>
            <button class="closeBtn" onclick={onclose}>✕</button>
        </div>
    </div>
{/if}

<style>
    .toolbar {
        position: fixed;
        top: 0;
        left: 0;
        right: 0;
        height: 44px;
        box-sizing: border-box;
        display: flex;
        align-items: center;
        gap: 8px;
        padding: 0 16px;
        background: rgba(10, 12, 18, 0.94);
        border-bottom: 1px solid #2a2e39;
        z-index: 1001;
        backdrop-filter: blur(8px);
    }
    .title {
        color: #f1f5f9;
        font-size: 12px;
        font-weight: 700;
        font-family: var(--mono);
        text-transform: uppercase;
        letter-spacing: 0.05em;
        flex: 1;
    }
    .actions {
        display: flex;
        align-items: center;
        gap: 8px;
    }
    .screenshotBtn {
        padding: 5px 12px;
        border: 1px solid #2a2e39;
        border-radius: 4px;
        background: transparent;
        color: #888;
        cursor: pointer;
        font-size: 11px;
        font-family: var(--mono);
        transition: background 0.15s, color 0.15s;
    }
    .screenshotBtn:hover { background: #1a1d26; color: #fff; }
    .closeBtn {
        background: none;
        border: none;
        color: #64748b;
        font-size: 18px;
        cursor: pointer;
        padding: 4px 8px;
        line-height: 1;
        transition: color 0.15s;
    }
    .closeBtn:hover { color: #f1f5f9; }
</style>
