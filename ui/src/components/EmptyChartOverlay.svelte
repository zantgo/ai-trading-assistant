<script lang="ts">
    // Empty-state overlay for charts that have neither historical nor
    // live data. Rendered when:
    //   - history payload was empty for the requested field, AND
    //   - WS hasn't delivered a value yet.
    // Visually identical to the warming footer so users recognize it.
    let { reason = 'no_history' }: { reason?: 'no_history' | 'warming' | 'live_only' } = $props();

    const message = $derived(reason === 'warming'
        ? 'AWAITING LIVE WARM-UP'
        : 'NO HISTORICAL DATA');
</script>

<div class="empty-overlay">
    <span class="empty-pill">{message}</span>
</div>

<style>
    .empty-overlay {
        position: absolute;
        inset: 0;
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 4;
        pointer-events: none;
    }
    .empty-pill {
        font-family: 'Courier New', monospace;
        font-size: 9px;
        font-weight: 700;
        letter-spacing: 0.06em;
        color: #ffb300;
        background: rgba(0, 0, 0, 0.7);
        border: 1px solid rgba(255, 179, 0, 0.4);
        border-radius: 3px;
        padding: 2px 8px;
        opacity: 0.85;
    }
</style>
