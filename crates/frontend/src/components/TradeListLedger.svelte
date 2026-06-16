<script lang="ts">
    import { getState } from '../state.svelte';
    import type { TradeJournalRecord } from '../state.svelte';

    const app = getState();

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

<div class="tl-layout">
    <div class="tl-header-ribbon">
        <div class="tl-ribbon-left">
            <h3 class="tl-title">TRADE LIST LEDGER</h3>
            <span class="tl-count">{app.tradeJournalRecords.length} journal entries</span>
        </div>
        <div class="tl-ribbon-right">
            <span class="tl-stat">
                <span class="tl-stat-label">Avg Execution Score:</span>
                <span class="tl-stat-value">{avgScore()}</span>
                <span class="tl-stat-out-of">/ 10.0</span>
            </span>
            <span class="tl-stat">
                <span class="tl-stat-label">Consecutive Losses:</span>
                <span class="tl-stat-value" class:tl-loss-count={consecutiveLosses() > 0}>{consecutiveLosses()}</span>
            </span>
        </div>
    </div>

    <div class="tl-table-wrap">
        <table class="tl-table">
            <thead>
                <tr>
                    <th>ID</th>
                    <th>Entry Date</th>
                    <th>Asset</th>
                    <th>Dir</th>
                    <th>Entry Reason</th>
                    <th>ROE</th>
                    <th>Score</th>
                    <th>AI Retrospective</th>
                </tr>
            </thead>
            <tbody>
                {#each app.tradeJournalRecords as trade (trade.id)}
                    <tr
                        class:tl-editing-row={editingId === trade.id}
                        ondblclick={() => openEdit(trade.id)}
                        title="Double-click to edit notes and score"
                    >
                        <td class="tl-mono tl-dim">{trade.id}</td>
                        <td class="tl-mono tl-dim">{formatDate(trade.entry_date)}</td>
                        <td class="tl-symbol">{trade.asset}</td>
                        <td class="tl-dir" class:tl-long={trade.direction === 'LONG'} class:tl-short={trade.direction === 'SHORT'}>
                            {trade.direction}
                        </td>
                        <td class="tl-reason">
                            {trade.entry_reason.length > 50 ? trade.entry_reason.substring(0, 50) + '...' : trade.entry_reason}
                        </td>
                        <td class="tl-mono">
                            <span class="tl-roe-badge" class:tl-roe-pos={trade.roe_percentage > 0} class:tl-roe-neg={trade.roe_percentage < 0}>
                                {formatPct(trade.roe_percentage)}
                            </span>
                        </td>
                        <td class="tl-mono">
                            <span class="tl-score-pill" class:tl-score-high={trade.execution_score >= 7} class:tl-score-mid={trade.execution_score >= 4 && trade.execution_score < 7} class:tl-score-low={trade.execution_score < 4}>
                                {trade.execution_score.toFixed(1)}
                            </span>
                        </td>
                        <td class="tl-analysis">{analysisPreview(trade.final_analysis)}</td>
                    </tr>

                    {#if editingId === trade.id}
                        <tr class="tl-edit-panel-row">
                            <td colspan="8">
                                <div class="tl-edit-panel">
                                    <div class="tl-edit-field">
                                        <label for="editNotes">Human Notes / Reflections:</label>
                                        <textarea
                                            id="editNotes"
                                            bind:value={editNotes}
                                            placeholder="Add your personal reflections on this trade..."
                                            rows="3"
                                        ></textarea>
                                    </div>
                                    <div class="tl-edit-field tl-edit-score-field">
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
                                    <div class="tl-edit-actions">
                                        <button class="tl-btn-save" onclick={saveEdit}>Save Changes</button>
                                        <button class="tl-btn-cancel" onclick={cancelEdit}>Cancel</button>
                                    </div>
                                </div>
                            </td>
                        </tr>
                    {/if}
                {/each}
            </tbody>
        </table>
        {#if app.tradeJournalRecords.length === 0}
            <div class="tl-empty">No trade journal entries yet. Closed trades will be automatically audited and appear here.</div>
        {/if}
    </div>

    <div class="tl-export-bar">
        <button class="tl-btn-export" onclick={() => app.exportJournalCSV()}>
            EXPORT LEDGER DATA (CSV)
        </button>
        <button class="tl-btn-export" onclick={() => app.exportJournalJSON()}>
            EXPORT LEDGER DATA (JSON)
        </button>
    </div>
</div>

<style>
    .tl-layout { max-width: 1400px; margin: 0 auto; width: 100%; padding: 16px; box-sizing: border-box; }
    .tl-header-ribbon {
        display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px;
        background: #1a1f2e; border: 1px solid #2a2e39; border-radius: 8px; padding: 12px 16px;
    }
    .tl-ribbon-left { display: flex; align-items: center; gap: 12px; }
    .tl-ribbon-right { display: flex; gap: 24px; }
    .tl-title { font-size: 12px; font-weight: 700; color: #64748b; text-transform: uppercase; letter-spacing: 0.05em; margin: 0; }
    .tl-count { font-size: 10px; color: #64748b; font-weight: 600; }
    .tl-stat { display: flex; align-items: baseline; gap: 4px; }
    .tl-stat-label { font-size: 10px; color: #64748b; font-weight: 600; text-transform: uppercase; }
    .tl-stat-value { font-size: 14px; font-weight: 700; color: #10b981; font-family: ui-monospace, monospace; }
    .tl-stat-out-of { font-size: 10px; color: #64748b; }
    .tl-loss-count { color: #ef4444; }

    .tl-table-wrap { overflow-x: auto; }
    .tl-table {
        width: 100%; border-collapse: collapse; font-size: 10px; color: #94a3b8;
        background: #131722; border: 1px solid #2a2e39; border-radius: 8px;
    }
    .tl-table thead { position: sticky; top: 0; background: #131722; z-index: 1; }
    .tl-table th {
        text-align: left; padding: 8px 10px; font-weight: 700; color: #64748b; text-transform: uppercase;
        letter-spacing: 0.04em; border-bottom: 2px solid #1e293b; font-size: 9px;
    }
    .tl-table td { padding: 6px 10px; border-bottom: 1px solid #0f131c; white-space: nowrap; }
    .tl-table tbody tr:hover { background: #1a1f2e; cursor: pointer; }
    .tl-editing-row { background: #1e293b !important; }
    .tl-mono { font-family: ui-monospace, monospace; font-size: 9px; }
    .tl-dim { color: #64748b; }
    .tl-symbol { font-weight: 700; color: #cbd5e1; }
    .tl-dir { font-weight: 700; text-transform: uppercase; font-size: 9px; }
    .tl-long { color: #10b981; }
    .tl-short { color: #ef4444; }
    .tl-reason { max-width: 180px; overflow: hidden; text-overflow: ellipsis; font-size: 9px; color: #94a3b8; }
    .tl-analysis { max-width: 280px; overflow: hidden; text-overflow: ellipsis; font-size: 9px; color: #94a3b8; line-height: 1.3; }

    .tl-roe-badge {
        font-size: 9px; font-weight: 700; padding: 2px 6px; border-radius: 3px;
        font-family: ui-monospace, monospace;
    }
    .tl-roe-pos { background: rgba(16,185,129,0.1); color: #10b981; }
    .tl-roe-neg { background: rgba(239,68,68,0.1); color: #ef4444; }

    .tl-score-pill {
        font-size: 9px; font-weight: 700; padding: 2px 8px; border-radius: 10px;
        font-family: ui-monospace, monospace;
    }
    .tl-score-high { background: rgba(16,185,129,0.12); color: #10b981; }
    .tl-score-mid { background: rgba(245,158,11,0.12); color: #f59e0b; }
    .tl-score-low { background: rgba(239,68,68,0.12); color: #ef4444; }

    .tl-edit-panel-row td { padding: 0; }
    .tl-edit-panel {
        background: #1a1f2e; border: 1px solid #3b82f6; border-radius: 6px;
        padding: 12px 16px; margin: 4px 10px; display: flex; flex-wrap: wrap; gap: 12px; align-items: flex-end;
    }
    .tl-edit-field { display: flex; flex-direction: column; gap: 4px; flex: 1; min-width: 200px; }
    .tl-edit-field label { font-size: 9px; font-weight: 600; color: #64748b; text-transform: uppercase; }
    .tl-edit-field textarea {
        background: #0f131c; border: 1px solid #2a2e39; border-radius: 4px; color: #cbd5e1;
        padding: 6px 8px; font-size: 10px; font-family: inherit; resize: vertical;
    }
    .tl-edit-field input {
        background: #0f131c; border: 1px solid #2a2e39; border-radius: 4px; color: #cbd5e1;
        padding: 6px 8px; font-size: 10px; font-family: ui-monospace, monospace; width: 80px;
    }
    .tl-edit-score-field { flex: 0 0 auto; min-width: 140px; }
    .tl-edit-actions { display: flex; gap: 8px; align-items: flex-end; }
    .tl-btn-save {
        background: #3b82f6; color: #fff; border: none; border-radius: 4px; padding: 6px 14px;
        font-size: 10px; font-weight: 600; cursor: pointer; text-transform: uppercase;
    }
    .tl-btn-save:hover { background: #2563eb; }
    .tl-btn-cancel {
        background: transparent; color: #64748b; border: 1px solid #2a2e39; border-radius: 4px;
        padding: 6px 14px; font-size: 10px; font-weight: 600; cursor: pointer; text-transform: uppercase;
    }
    .tl-btn-cancel:hover { color: #cbd5e1; border-color: #64748b; }

    .tl-export-bar {
        display: flex; justify-content: space-between; margin-top: 12px; gap: 12px;
    }
    .tl-btn-export {
        flex: 1; background: rgba(59,130,246,0.08); color: #60a5fa; border: 1px solid rgba(59,130,246,0.2);
        border-radius: 6px; padding: 10px; font-size: 10px; font-weight: 700; cursor: pointer;
        text-transform: uppercase; letter-spacing: 0.04em;
    }
    .tl-btn-export:hover { background: rgba(59,130,246,0.14); }

    .tl-empty { font-size: 11px; color: #64748b; text-align: center; padding: 40px; font-style: italic; }
</style>
