<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import type { TradeJournalRecord } from '../types';
    import styles from './TradeListLedger.module.css';

    const app = useAppStore();

    let editingId = $state<number | null>(null);
    let editNotes = $state('');
    let editScore = $state(0);

    $effect(() => {
        app.fetchTradeJournal(100);
    });

    function avgScore(): string {
        const records = app.tradeJournalRecords;
        if (records.length === 0) return '--';
        const sum = records.reduce((a, r) => a + r.execution_score, 0);
        return (sum / records.length).toFixed(1);
    }

    function consecutiveLosses(): number {
        const records = app.tradeJournalRecords;
        let count = 0;
        for (const r of records) {
            if (r.realized_pnl < 0) count++;
            else break;
        }
        return count;
    }

    function formatUsd(v: number): string {
        if (Math.abs(v) >= 1000) return '$' + v.toLocaleString('en-US', { maximumFractionDigits: 0 });
        return '$' + v.toFixed(2);
    }
    function formatPct(v: number): string { return (v >= 0 ? '+' : '') + v.toFixed(2) + '%'; }
    function formatDate(ts: string): string {
        if (!ts || ts === '0') return '--';
        return ts.replace(' ', '\n').substring(0, 16);
    }

    function openEdit(id: number) {
        const rec = app.tradeJournalRecords.find(r => r.id === id);
        if (!rec) return;
        editingId = id;
        editNotes = rec.human_notes || '';
        editScore = rec.execution_score;
    }

    async function saveEdit() {
        if (editingId === null) return;
        await app.updateJournalNotes(editingId, editNotes, editScore);
        editingId = null;
    }

    function cancelEdit() {
        editingId = null;
    }

    function analysisPreview(text: string): string {
        if (!text) return '--';
        return text.length > 120 ? text.substring(0, 120) + '...' : text;
    }
</script>

<div class={styles.tlLayout}>
    <div class={styles.tlHeaderRibbon}>
        <div class={styles.tlRibbonLeft}>
            <h3 class={styles.tlTitle}>TRADE LIST LEDGER</h3>
            <span class={styles.tlCount}>{app.tradeJournalRecords.length} journal entries</span>
        </div>
        <div class={styles.tlRibbonRight}>
            <span class={styles.tlStat}>
                <span class={styles.tlStatLabel}>Avg Execution Score:</span>
                <span class={styles.tlStatValue}>{avgScore()}</span>
                <span class={styles.tlStatOutOf}>/ 10.0</span>
            </span>
            <span class={styles.tlStat}>
                <span class={styles.tlStatLabel}>Consecutive Losses:</span>
                <span class="{styles.tlStatValue} {consecutiveLosses() > 0 ? styles.tlLossCount : ''}">{consecutiveLosses()}</span>
            </span>
        </div>
    </div>

    <div class={styles.tlTableWrap}>
        <table class={styles.tlTable}>
            <thead>
                <tr>
                    <th>ID</th>
                    <th>Entry Date</th>
                    <th>Asset</th>
                    <th>Dir</th>
                    <th>Entry Reason</th>
                    <th>ROE</th>
                    <th>Score</th>
                    <th>Retrospective</th>
                </tr>
            </thead>
            <tbody>
                {#each app.tradeJournalRecords as trade (trade.id)}
                    <tr
                        class={editingId === trade.id ? styles.tlEditingRow : ''}
                        ondblclick={() => openEdit(trade.id)}
                        title="Double-click to edit notes and score"
                    >
                        <td class="{styles.tlMono} {styles.tlDim}">{trade.id}</td>
                        <td class="{styles.tlMono} {styles.tlDim}">{formatDate(trade.entry_date)}</td>
                        <td class={styles.tlSymbol}>{trade.asset}</td>
                        <td class="{styles.tlDir} {trade.direction === 'LONG' ? styles.tlLong : styles.tlShort}">
                            {trade.direction}
                        </td>
                        <td class={styles.tlReason}>
                            {trade.entry_reason.length > 50 ? trade.entry_reason.substring(0, 50) + '...' : trade.entry_reason}
                        </td>
                        <td class={styles.tlMono}>
                            <span class="{styles.tlRoeBadge} {trade.roe_percentage > 0 ? styles.tlRoePos : styles.tlRoeNeg}">
                                {formatPct(trade.roe_percentage)}
                            </span>
                        </td>
                        <td class={styles.tlMono}>
                            <span class="{styles.tlScorePill} {trade.execution_score >= 7 ? styles.tlScoreHigh : trade.execution_score >= 4 ? styles.tlScoreMid : styles.tlScoreLow}">
                                {trade.execution_score.toFixed(1)}
                            </span>
                        </td>
                        <td class={styles.tlAnalysis}>{analysisPreview(trade.final_analysis)}</td>
                    </tr>

                    {#if editingId === trade.id}
                        <tr class={styles.tlEditPanelRow}>
                            <td colspan="8">
                                <div class={styles.tlEditPanel}>
                                    <div class={styles.tlEditField}>
                                        <label for="editNotes">Human Notes / Reflections:</label>
                                        <textarea
                                            id="editNotes"
                                            bind:value={editNotes}
                                            placeholder="Add your personal reflections on this trade..."
                                            rows="3"
                                        ></textarea>
                                    </div>
                                    <div class="{styles.tlEditField} {styles.tlEditScoreField}">
                                        <label for="editScore">Override Execution Score (0.0 - 10.0):</label>
                                        <input
                                            id="editScore"
                                            type="number"
                                            step="0.1"
                                            min="0"
                                            max="10"
                                            bind:value={editScore}
                                        />
                                    </div>
                                    <div class={styles.tlEditActions}>
                                        <button class={styles.tlBtnSave} onclick={saveEdit}>Save Changes</button>
                                        <button class={styles.tlBtnCancel} onclick={cancelEdit}>Cancel</button>
                                    </div>
                                </div>
                            </td>
                        </tr>
                    {/if}
                {/each}
            </tbody>
        </table>
        {#if app.tradeJournalRecords.length === 0}
            <div class={styles.tlEmpty}>No trade journal entries yet. Closed trades will be automatically audited and appear here.</div>
        {/if}
    </div>

    <div class={styles.tlExportBar}>
        <button class={styles.tlBtnExport} onclick={() => app.exportJournalCSV()}>
            EXPORT LEDGER DATA (CSV)
        </button>
        <button class={styles.tlBtnExport} onclick={() => app.exportJournalJSON()}>
            EXPORT LEDGER DATA (JSON)
        </button>
    </div>
</div>

