<script lang="ts">
    import { untrack } from 'svelte';
    import type { TriggerModeConfig, TriggerModeUnion } from '../../types';
    import styles from './settings.module.css';

    const SUPPORTED_EVENTS = [
        { key: 'squeeze_release', label: 'Squeeze Release' },
        { key: 'sr_flip', label: 'S/R Role Flip' },
        { key: 'ema200_cross', label: 'Slow EMA Cross' },
        { key: 'confirmed_divergence', label: 'Confirmed Divergence' },
    ];

    let {
        onchange,
        initial,
    }: { onchange?: (c: { trigger: TriggerModeConfig }) => void; initial?: { trigger: TriggerModeConfig } | null } = $props();

    let triggerMode = $state<TriggerModeUnion>(untrack(() => initial?.trigger?.mode ?? 'interval'));
    let intervalSeconds = $state<number>(
        untrack(() => initial?.trigger?.mode === 'interval' ? (initial.trigger as any).seconds : 900),
    );
    let candleTimeframe = $state<string>(
        untrack(() => initial?.trigger?.mode === 'candle_close' ? (initial.trigger as any).timeframe : 'slow'),
    );
    let candleCount = $state<number>(
        untrack(() => initial?.trigger?.mode === 'candle_close' ? (initial.trigger as any).count : 3),
    );
    let selectedEvents = $state<string[]>(
        untrack(() => initial?.trigger?.mode === 'event_driven' ? [...(initial.trigger as any).events] : []),
    );

    function toggleEvent(eventKey: string) {
        if (selectedEvents.includes(eventKey)) {
            selectedEvents = selectedEvents.filter((e) => e !== eventKey);
        } else {
            selectedEvents = [...selectedEvents, eventKey];
        }
        emit();
    }

    function emit() {
        let trigger: TriggerModeConfig;
        if (triggerMode === 'interval') {
            trigger = { mode: 'interval', seconds: intervalSeconds };
        } else if (triggerMode === 'candle_close') {
            trigger = { mode: 'candle_close', timeframe: candleTimeframe, count: candleCount };
        } else {
            trigger = { mode: 'event_driven', events: selectedEvents };
        }
        onchange?.({ trigger });
    }
</script>

<div class={styles.panel}>
    <h4 class={styles.panelTitle}>Trigger Setup</h4>
    <p class={styles.panelDesc}>Configure when policy triggers fire (time interval, candle close, or named events).</p>

    <div class={styles.fieldGroup}>
        <label class={styles.fieldLabel} for="tc-trigger-mode">Trigger Mode</label>
        <select id="tc-trigger-mode" class={styles.select} value={triggerMode} onchange={(e) => { triggerMode = e.currentTarget.value as TriggerModeUnion; emit(); }}>
            <option value="interval">Time Interval</option>
            <option value="candle_close">Candle Close</option>
            <option value="event_driven">Event Driven</option>
        </select>
    </div>

    {#if triggerMode === 'interval'}
        <div class={styles.fieldGroup}>
            <label class={styles.fieldLabel} for="tc-interval-seconds">Interval (seconds)</label>
            <input id="tc-interval-seconds" type="number" min="30" max="86400" step="30" value={intervalSeconds} oninput={(e) => { intervalSeconds = parseInt(e.currentTarget.value) || 900; emit(); }} class={styles.input} />
        </div>
    {/if}

    {#if triggerMode === 'candle_close'}
        <div class={styles.fieldRow}>
            <div class={styles.fieldGroup}>
                <label class={styles.fieldLabel} for="tc-candle-timeframe">Timeframe</label>
                <select id="tc-candle-timeframe" class={styles.select} value={candleTimeframe} onchange={(e) => { candleTimeframe = e.currentTarget.value; emit(); }}>
                    <option value="micro">Micro</option>
                    <option value="fast">Fast</option>
                    <option value="slow">Slow</option>
                    <option value="macro">Macro</option>
                </select>
            </div>
            <div class={styles.fieldGroup}>
                <label class={styles.fieldLabel} for="tc-candle-count">Candle Count</label>
                <input id="tc-candle-count" type="number" min="1" max="100" step="1" value={candleCount} oninput={(e) => { candleCount = parseInt(e.currentTarget.value) || 3; emit(); }} class={styles.input} />
            </div>
        </div>
    {/if}

    {#if triggerMode === 'event_driven'}
        <div class={styles.eventGrid}>
            {#each SUPPORTED_EVENTS as ev}
                <label class={styles.eventCheck}>
                    <input type="checkbox" checked={selectedEvents.includes(ev.key)} onchange={() => toggleEvent(ev.key)} />
                    <span>{ev.label}</span>
                </label>
            {/each}
        </div>
    {/if}
</div>
