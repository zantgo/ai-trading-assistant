<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import { createInstance } from '../lib/api.svelte';
    import styles from './Sidebar.module.css';
    const app = useAppStore();

    let collapsed = $state(false);
    let newPairInput = $state('');
    let showAddInput = $state(false);
    let addError = $state<string | null>(null);
    let addLoading = $state(false);

    function selectInstance(pairKey: string) {
        app.activeTab = pairKey;
        app.currentGlobalView = 'workspace';
    }

    async function confirmAdd() {
        const symbol = newPairInput.trim().toUpperCase();
        if (symbol.length < 2 || symbol.length > 10) {
            addError = 'Enter a symbol between 2 and 10 characters.';
            return;
        }

        addLoading = true;
        addError = null;
        const result = await createInstance(symbol, app.quote);
        addLoading = false;

        if (!result.ok) {
            addError = result.error || 'Failed to add instance.';
            return;
        }

        app.initInstance(symbol);
        app.activeTab = app.pairKeyFor(symbol);
        newPairInput = '';
        showAddInput = false;
    }

    function cancelAdd() {
        showAddInput = false;
        newPairInput = '';
        addError = null;
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
                        oninput={() => { if (addError) addError = null; }}
                        onkeydown={(e) => { if (e.key === 'Enter') confirmAdd(); if (e.key === 'Escape') cancelAdd(); }}
                        class={styles.addPairInput}
                        autofocus
                    />
                    <span class={styles.addPairQuote} title="Settlement currency is set by your session">-{app.quote}</span>
                    <button class={styles.addPairConfirm} onclick={confirmAdd} disabled={addLoading}>{addLoading ? '…' : '+'}</button>
                    <button class={styles.addPairCancel} onclick={cancelAdd}>×</button>
                </div>
                {#if addError}
                    <div class={styles.addPairError} role="alert">⚠ {addError}</div>
                {/if}
            {:else}
                <button class={styles.addPairBtn} onclick={() => showAddInput = true}>
                    + Add Instance
                </button>
            {/if}
        </div>
    {/if}
</div>
