<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import type { InstanceSummary } from '../types';
    import styles from './InstanceList.module.css';

    interface Props {
        onNavigate?: (view: string) => void;
    }
    let { onNavigate }: Props = $props();

    const app = useAppStore();
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
                app.initInstance(base);
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

    function navigateToInstance(pair: string, symbol: string) {
        if (!app.instancesMap[pair]) {
            app.initInstance(symbol);
        }
        app.activeTab = pair;
        app.currentGlobalView = 'workspace';
    }

    async function handleAction(instanceId: string, action: 'pause' | 'stop' | 'delete', pair?: string) {
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
            if (action === 'delete' && pair) {
                app.removeInstance(pair);
            }
            await fetchInstances();
        } finally {
            const next = { ...actionLoading };
            delete next[instanceId];
            actionLoading = next;
        }
    }

    function statusClass(status: string) {
        switch (status) {
            case 'running': return styles.statusRunning;
            case 'paused': return styles.statusPaused;
            case 'stopped': return styles.statusStopped;
            default: return '';
        }
    }

    $effect(() => { fetchInstances(); });
</script>

<div class={styles.instancesView}>
    <div class={styles.instancesHeader}>
        <h2>All Instances</h2>
        <span class={styles.instancesCount}>{totalCount} / {maxCount}</span>
    </div>

    <!-- Add Instance -->
    <div class={styles.addInstanceBar}>
        <input
            type="text"
            class={styles.addInput}
            placeholder="Base (e.g. BTC)"
            bind:value={newBase}
            maxlength="10"
            onkeydown={(e: KeyboardEvent) => { if (e.key === 'Enter') handleCreate(); }}
        />
        <input
            type="text"
            class="{styles.addInput} {styles.short}"
            placeholder="Quote"
            bind:value={newQuote}
            maxlength="10"
            onkeydown={(e: KeyboardEvent) => { if (e.key === 'Enter') handleCreate(); }}
        />
        <button class={styles.addBtn} onclick={handleCreate} disabled={addLoading || !newBase.trim()}>
            {addLoading ? '...' : '+ Create'}
        </button>
    </div>

    {#if loading}
        <div class={styles.loadingRow}>Loading instances...</div>
    {:else if error}
        <div class={styles.errorRow}>{error}</div>
    {:else if instances.length === 0}
        <div class={styles.emptyRow}>No active instances. Create one above.</div>
    {:else}
        <div class={styles.instancesTable}>
            <div class={styles.tableHeader}>
                <span class="col-id">ID</span>
                <span class="col-pair">Pair</span>
                <span class="col-status">Status</span>
                <span class="col-capital">Capital</span>
                <span class="col-equity">Equity</span>
                <span class="col-losses">Losses</span>
                <span class={styles.colActions}>Actions</span>
            </div>
            {#each instances as inst (inst.id)}
                <div class={styles.tableRow}>
                    <span class="col-id" title={inst.id}>{inst.id.substring(0, 12)}</span>
                    <span class="col-pair">{inst.pair}</span>
                    <span class="col-status">
                        <span class="{styles.statusDot} {statusClass(inst.status)}"></span>
                        {inst.status}
                    </span>
                    <span class="col-capital">{inst.initial_capital.toFixed(2)}</span>
                    <span class="col-equity">{inst.current_equity.toFixed(2)}</span>
                    <span class="col-losses {inst.consecutive_losses >= 5 ? styles.lossDanger : inst.consecutive_losses >= 3 ? styles.lossWarn : ''}">
                        {inst.consecutive_losses}
                    </span>
                    <span class={styles.colActions}>
                        <button
                            class="{styles.actionBtnSm} {styles.viewBtn}"
                            onclick={() => navigateToInstance(inst.pair, inst.symbol)}
                            title="View cockpit"
                        >📈</button>
                        {#if inst.status !== 'stopped'}
                            <button
                                class="{styles.actionBtnSm} {styles.pauseBtn}"
                                onclick={() => handleAction(inst.id, 'pause')}
                                disabled={actionLoading[inst.id] !== undefined || inst.status === 'paused'}
                                title="Pause"
                            >⏸</button>
                            <button
                                class="{styles.actionBtnSm} {styles.stopBtn}"
                                onclick={() => handleAction(inst.id, 'stop')}
                                disabled={actionLoading[inst.id] !== undefined}
                                title="Stop"
                            >⏹</button>
                        {/if}
                        <button
                            class="{styles.actionBtnSm} {styles.deleteBtn}"
                            onclick={() => handleAction(inst.id, 'delete', inst.pair)}
                            disabled={actionLoading[inst.id] !== undefined}
                            title="Delete"
                        >🗑</button>
                    </span>
                </div>
            {/each}
        </div>
    {/if}
</div>

