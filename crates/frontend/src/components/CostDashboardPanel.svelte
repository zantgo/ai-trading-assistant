<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import styles from '../App.module.css';

    const app = useAppStore();
    let { pair }: { pair: { symbol: string } } = $props();
</script>

<div class={styles.workspaceInnerContent + " " + 'animate-fade'}>
    <div class={styles.costDashboard}>
        <div class={styles.costHeader}>
            <h2 class={styles.costTitle}>AI Token Cost Analysis — {pair.symbol}</h2>
            <button class={styles.costRefreshBtn} onclick={() => app.fetchCostEstimate()} disabled={app.costLoading}>
                {app.costLoading ? 'Loading...' : 'Refresh'}
            </button>
        </div>

        <div class={styles.costCardsRow}>
            <div class={styles.costCard + " " + styles.costCardDaily}>
                <span class={styles.costCardLabel}>Projected Daily</span>
                <span class={styles.costCardValue}>${app.costDailyProjected.toFixed(4)}</span>
                <span class={styles.costCardSub}>{app.costRunsPerDay.toFixed(1)} runs/day</span>
            </div>
            <div class={styles.costCard + " " + styles.costCardWeekly}>
                <span class={styles.costCardLabel}>Projected Weekly</span>
                <span class={styles.costCardValue}>${app.costWeeklyProjected.toFixed(4)}</span>
                <span class={styles.costCardSub}>7 days</span>
            </div>
            <div class={styles.costCard + " " + styles.costCardMonthly}>
                <span class={styles.costCardLabel}>Projected Monthly</span>
                <span class={styles.costCardValue}>${app.costMonthlyProjected.toFixed(4)}</span>
                <span class={styles.costCardSub}>30 days</span>
            </div>
        </div>

        <div class={styles.costDetailsGrid}>
            <div class={styles.costDetailBox}>
                <h4 class={styles.costDetailTitle}>Pricing Configuration</h4>
                <div class={styles.costDetailRow}>
                    <span>Input (per 1M tokens)</span>
                    <span class={styles.mono}>${app.costPriceInput.toFixed(4)}</span>
                </div>
                <div class={styles.costDetailRow}>
                    <span>Output (per 1M tokens)</span>
                    <span class={styles.mono}>${app.costPriceOutput.toFixed(4)}</span>
                </div>
                <div class={styles.costDetailRow}>
                    <span>Prompt Interval</span>
                    <span class={styles.mono}>{app.costIntervalSecs}s ({Math.round(app.costIntervalSecs / 60)} min)</span>
                </div>
            </div>
            <div class={styles.costDetailBox}>
                <h4 class={styles.costDetailTitle}>Per-Run Token Estimate</h4>
                <div class={styles.costDetailRow}>
                    <span>Input Tokens</span>
                    <span class={styles.mono}>{app.costTokensPerRunInput.toLocaleString()}</span>
                </div>
                <div class={styles.costDetailRow}>
                    <span>Output Tokens</span>
                    <span class={styles.mono}>{app.costTokensPerRunOutput.toLocaleString()}</span>
                </div>
                <div class={styles.costDetailRow}>
                    <span>Total per Run</span>
                    <span class={styles.mono}>{(app.costTokensPerRunInput + app.costTokensPerRunOutput).toLocaleString()}</span>
                </div>
            </div>
            <div class={styles.costDetailBox}>
                <h4 class={styles.costDetailTitle}>Actual Usage Tracked</h4>
                <div class={styles.costDetailRow}>
                    <span>Input Tokens Used</span>
                    <span class={styles.mono}>{app.costActualInputTokens.toLocaleString()}</span>
                </div>
                <div class={styles.costDetailRow}>
                    <span>Output Tokens Used</span>
                    <span class={styles.mono}>{app.costActualOutputTokens.toLocaleString()}</span>
                </div>
                <div class={styles.costDetailRow} style="border-top: 1px solid #2a2e39; padding-top: 8px; margin-top: 4px;">
                    <span style="font-weight: 700; color: #e2e8f0;">Actual Spend</span>
                    <span class={styles.mono} style="color: #f59e0b; font-weight: 700;">${app.costActualTotal.toFixed(6)}</span>
                </div>
            </div>
        </div>

        <div class={styles.costInfoBox}>
            <p>
                <strong>How it works:</strong> The calculator uses your AI model's per-1M-token pricing (configurable in Workspace Settings)
                combined with the automation prompt interval for this pair. It estimates ~{app.costTokensPerRunInput.toLocaleString()} input + ~{app.costTokensPerRunOutput.toLocaleString()} output tokens
                per analysis run (28 indicator agents × 512 tokens + 1 orchestrator × 1024 tokens).
                Actual usage is tracked from real LLM API responses and shown above.
            </p>
        </div>
    </div>
</div>
