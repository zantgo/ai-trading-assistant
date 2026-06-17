<script lang="ts">
    import { getState } from '../state.svelte';
    import type { InstanceSummary } from '../state.svelte';

    interface Props {
        onNavigate?: (view: string) => void;
    }
    let { onNavigate }: Props = $props();

    const app = getState();
    let instances = $state<InstanceSummary[]>([]);
    let totalCount = $state(0);
    let maxCount = $state(100);
    let loading = $state(true);
    let error = $state<string | null>(null);
    let actionLoading = $state<Record<string, string>>({});

    let newBase = $state('');
    let newQuote = $state('USDT');
    let addLoading = $state(false);

    async function fetchInstances() {
        loading = true;
        error = null;
        try {
            const res = await fetch('/api/instances');
            if (res.ok) {
                const data = await res.json();
                instances = data.instances || [];
                totalCount = data.total_count || instances.length;
                maxCount = data.max_count || 100;
            } else {
                error = 'Failed to fetch instances';
            }
        } catch (e: any) {
            error = e.message || 'Network error';
        } finally {
            loading = false;
        }
    }

    async function handleCreate() {
        const base = newBase.trim().toUpperCase();
        const quote = newQuote.trim().toUpperCase();
        if (!base) return;

        addLoading = true;
        try {
            const res = await fetch('/api/instances', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ base, quote }),
            });
            const data = await res.json();
            if (res.ok) {
                newBase = '';
                await fetchInstances();
            } else {
                alert(data.error || data || 'Failed to create instance');
            }
        } catch (e: any) {
            alert(e.message || 'Network error');
        } finally {
            addLoading = false;
        }
    }

    async function handleAction(instanceId: string, action: 'pause' | 'stop' | 'delete') {
        const verb = action === 'delete' ? 'DELETE' : 'POST';
        let url = '';
        if (action === 'delete') {
            url = `/api/instances/${encodeURIComponent(instanceId)}`;
        } else {
            url = `/api/instances/${encodeURIComponent(instanceId)}/${action}`;
        }

        actionLoading = { ...actionLoading, [instanceId]: action };
        try {
            await fetch(url, { method: verb });
            await fetchInstances();
        } finally {
            const next = { ...actionLoading };
            delete next[instanceId];
            actionLoading = next;
        }
    }

    function statusClass(status: string) {
        switch (status) {
            case 'running': return 'status-running';
            case 'paused': return 'status-paused';
            case 'stopped': return 'status-stopped';
            default: return '';
        }
    }

    $effect(() => { fetchInstances(); });
</script>

<div class="instances-view">
    <div class="instances-header">
        <h2>All Instances</h2>
        <span class="instances-count">{totalCount} / {maxCount}</span>
    </div>

    <!-- Add Instance -->
    <div class="add-instance-bar">
        <input
            type="text"
            class="add-input"
            placeholder="Base (e.g. BTC)"
            bind:value={newBase}
            maxlength="10"
            onkeydown={(e: KeyboardEvent) => { if (e.key === 'Enter') handleCreate(); }}
        />
        <input
            type="text"
            class="add-input short"
            placeholder="Quote"
            bind:value={newQuote}
            maxlength="10"
            onkeydown={(e: KeyboardEvent) => { if (e.key === 'Enter') handleCreate(); }}
        />
        <button class="add-btn" onclick={handleCreate} disabled={addLoading || !newBase.trim()}>
            {addLoading ? '...' : '+ Create'}
        </button>
    </div>

    {#if loading}
        <div class="loading-row">Loading instances...</div>
    {:else if error}
        <div class="error-row">{error}</div>
    {:else if instances.length === 0}
        <div class="empty-row">No active instances. Create one above.</div>
    {:else}
        <div class="instances-table">
            <div class="table-header">
                <span class="col-id">ID</span>
                <span class="col-pair">Pair</span>
                <span class="col-status">Status</span>
                <span class="col-capital">Capital</span>
                <span class="col-equity">Equity</span>
                <span class="col-losses">Losses</span>
                <span class="col-actions">Actions</span>
            </div>
            {#each instances as inst (inst.id)}
                <div class="table-row">
                    <span class="col-id" title={inst.id}>{inst.id.substring(0, 12)}</span>
                    <span class="col-pair">{inst.pair}</span>
                    <span class="col-status">
                        <span class="status-dot {statusClass(inst.status)}"></span>
                        {inst.status}
                    </span>
                    <span class="col-capital">{inst.initial_capital.toFixed(2)}</span>
                    <span class="col-equity">{inst.current_equity.toFixed(2)}</span>
                    <span class="col-losses" class:loss-warn={inst.consecutive_losses >= 3} class:loss-danger={inst.consecutive_losses >= 5}>
                        {inst.consecutive_losses}
                    </span>
                    <span class="col-actions">
                        {#if inst.status !== 'stopped'}
                            <button
                                class="action-btn-sm pause-btn"
                                onclick={() => handleAction(inst.id, 'pause')}
                                disabled={actionLoading[inst.id] !== undefined || inst.status === 'paused'}
                                title="Pause"
                            >⏸</button>
                            <button
                                class="action-btn-sm stop-btn"
                                onclick={() => handleAction(inst.id, 'stop')}
                                disabled={actionLoading[inst.id] !== undefined}
                                title="Stop"
                            >⏹</button>
                        {/if}
                        <button
                            class="action-btn-sm delete-btn"
                            onclick={() => handleAction(inst.id, 'delete')}
                            disabled={actionLoading[inst.id] !== undefined}
                            title="Delete"
                        >🗑</button>
                    </span>
                </div>
            {/each}
        </div>
    {/if}
</div>

<style>
    .instances-view {
        padding: 1.5rem;
        color: #cbd5e1;
    }
    .instances-header {
        display: flex;
        align-items: baseline;
        gap: 0.75rem;
        margin-bottom: 1rem;
    }
    .instances-header h2 {
        margin: 0;
        font-size: 1.2rem;
        color: #e0e0ff;
    }
    .instances-count {
        font-size: 0.8rem;
        color: #64748b;
    }
    .add-instance-bar {
        display: flex;
        gap: 0.5rem;
        margin-bottom: 1rem;
    }
    .add-input {
        flex: 1;
        padding: 0.45rem 0.6rem;
        background: #1e1e3a;
        border: 1px solid #333355;
        border-radius: 6px;
        color: #e0e0ff;
        font-size: 0.85rem;
        outline: none;
    }
    .add-input:focus { border-color: #5b7fff; }
    .add-input.short { flex: 0 0 100px; }
    .add-btn {
        padding: 0.45rem 1rem;
        background: #5b7fff;
        border: none;
        border-radius: 6px;
        color: white;
        font-size: 0.85rem;
        font-weight: 600;
        cursor: pointer;
        white-space: nowrap;
    }
    .add-btn:disabled { opacity: 0.5; cursor: not-allowed; }
    .loading-row, .error-row, .empty-row {
        text-align: center;
        padding: 2rem;
        color: #64748b;
    }
    .error-row { color: #ff6666; }
    .instances-table {
        border: 1px solid #2a2a4a;
        border-radius: 8px;
        overflow: hidden;
    }
    .table-header, .table-row {
        display: grid;
        grid-template-columns: 100px 100px 90px 90px 90px 60px 100px;
        align-items: center;
        padding: 0.55rem 0.8rem;
        font-size: 0.82rem;
    }
    .table-header {
        background: #1a1a35;
        color: #8888aa;
        font-weight: 600;
        text-transform: uppercase;
        font-size: 0.7rem;
        letter-spacing: 0.5px;
    }
    .table-row {
        background: #14142a;
        border-top: 1px solid #1e1e3a;
    }
    .table-row:hover { background: #1a1a35; }
    .status-dot {
        display: inline-block;
        width: 8px;
        height: 8px;
        border-radius: 50%;
        margin-right: 4px;
    }
    .status-running { background: #22c55e; }
    .status-paused { background: #f59e0b; }
    .status-stopped { background: #ef4444; }
    .loss-warn { color: #f59e0b; font-weight: 600; }
    .loss-danger { color: #ef4444; font-weight: 700; }
    .col-actions {
        display: flex;
        gap: 4px;
    }
    .action-btn-sm {
        padding: 2px 6px;
        border: 1px solid #333355;
        border-radius: 4px;
        background: #1e1e3a;
        color: #8888aa;
        cursor: pointer;
        font-size: 0.8rem;
    }
    .action-btn-sm:hover:not(:disabled) { background: #2a2a50; color: #cbd5e1; }
    .action-btn-sm:disabled { opacity: 0.3; cursor: not-allowed; }
    .pause-btn:hover:not(:disabled) { border-color: #f59e0b; color: #f59e0b; }
    .stop-btn:hover:not(:disabled) { border-color: #ef4444; color: #ef4444; }
    .delete-btn:hover:not(:disabled) { border-color: #ef4444; color: #ef4444; }
</style>
