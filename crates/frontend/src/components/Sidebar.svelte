<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import styles from './Sidebar.module.css';
    const app = useAppStore();

    let collapsed = $state(false);
    let newPairInput = $state('');
    let showAddInput = $state(false);

    function selectInstance(pairKey: string) {
        app.activeTab = pairKey;
        app.currentGlobalView = 'workspace';
    }

    function confirmAdd() {
        const raw = newPairInput.trim().toUpperCase();
        if (raw.length < 2 || raw.length > 10) return;

        const symbol = raw;
        const pairKey = `${symbol}-USDT`;

        app.initInstance(symbol);

        // Create via instance API
        fetch('/api/instances', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ base: symbol, quote: 'USDT' }),
        }).then(() => {
            app.activeTab = pairKey;
        }).catch(console.error);

        newPairInput = '';
        showAddInput = false;
    }

    async function removeInstance(pairKey: string) {
        await fetch(`/api/instances/by-pair/${encodeURIComponent(pairKey)}`, { method: 'DELETE' }).catch(console.error);
        app.removeInstance(pairKey);
        const remaining = Object.keys(app.instancesMap);
        if (remaining.length > 0 && pairKey === app.activeTab) {
            app.activeTab = remaining[0];
        }
    }

    function pairLabel(key: string): string {
        const parts = key.split('-');
        return `<span class="pair-label-exchange">${parts[0]}</span><span class="pair-label-symbol">${parts[1] || key}</span>`;
    }

    function shortName(key: string): string {
        const parts = key.split('-');
        return (parts[1] || key).substring(0, 4).toUpperCase();
    }
</script>

<div class="{styles.sidebarContainer} {collapsed ? styles.collapsed : ''}">
    <button class={styles.sidebarToggleBtn} onclick={() => collapsed = !collapsed} title={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}>
        {collapsed ? '▶' : '◀'}
    </button>

    {#if !collapsed}
        <div class={styles.sidebarHeader}>
            <span class={styles.sidebarLogo}>ACTIVE INSTANCES</span>
            <span class={styles.sidebarStatus}>{@html app.isConnected ? '<span class="status-live">● LIVE</span>' : '<span class="status-offline">● OFFLINE</span>'}</span>
        </div>
    {:else}
        <div class="{styles.sidebarHeader} {styles.collapsedHeader}">
            <span class={styles.sidebarLogoSmall}>AT</span>
        </div>
    {/if}

    <div class={styles.sidebarPairsList}>
        {#each Object.keys(app.instancesMap) as pairKey}
            <button
                class="{styles.pairItem} {pairKey === app.activeTab ? styles.active : ''}"
                onclick={() => selectInstance(pairKey)}
                title={pairKey}
            >
                {#if collapsed}
                    <span class={styles.pairShort}>{shortName(pairKey)}</span>
                {:else}
                    <span class={styles.pairName}>{@html pairLabel(pairKey)}</span>
                    <span class="{styles.pairStatusDot} {app.instancesMap[pairKey].isConnected ? styles.connected : ''}"></span>
                    <span class={styles.pairRemoveBtn} role="button" tabindex="0" onclick={(e) => { e.stopPropagation(); removeInstance(pairKey); }} onkeydown={(e) => { if (e.key === 'Enter') { e.stopPropagation(); removeInstance(pairKey); } }} title="Remove instance">×</span>
                {/if}
            </button>
        {/each}
    </div>

    {#if !collapsed}
        <div class={styles.sidebarAddSection}>
            {#if showAddInput}
                <div class={styles.addPairInputGroup}>
                        <!-- svelte-ignore a11y_autofocus -->
                        <input
                        type="text"
                        placeholder="e.g. ETH"
                        bind:value={newPairInput}
                        onkeydown={(e) => { if (e.key === 'Enter') confirmAdd(); }}
                        class={styles.addPairInput}
                        autofocus
                    />
                    <button class={styles.addPairConfirm} onclick={confirmAdd}>+</button>
                    <button class={styles.addPairCancel} onclick={() => { showAddInput = false; newPairInput = ''; }}>×</button>
                </div>
            {:else}
                <button class={styles.addPairBtn} onclick={() => showAddInput = true}>
                    + Add Instance
                </button>
            {/if}
        </div>
    {/if}
</div>
