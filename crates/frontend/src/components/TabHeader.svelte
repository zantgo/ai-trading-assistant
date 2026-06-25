<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import styles from './TabHeader.module.css';
    const app = useAppStore();

    let newPairInput = $state('');
    let showAddInput = $state(false);

    function selectTab(pairKey: string) {
        app.activeTab = pairKey;
    }

    function confirmAdd() {
        const raw = newPairInput.trim().toUpperCase();
        if (raw.length < 2 || raw.length > 10) return;

        // Create instance
        const symbol = raw;
        const pairKey = `${symbol}-USDT`;

        app.initInstance(symbol);
        fetch(`/api/instances`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ base: symbol, quote: 'USDT' }),
        }).then(() => {
            app.activeTab = pairKey;
        }).catch(console.error);

        newPairInput = '';
        showAddInput = false;
    }

    function removeInstance(pairKey: string) {
        // TODO: Phase 4 — delete via /api/instances with instance ID lookup
        app.removeInstance(pairKey);
        const remaining = Object.keys(app.instancesMap);
        if (remaining.length > 0) {
            if (pairKey === app.activeTab) {
                app.activeTab = remaining[0];
            }
        }
    }
</script>

<div class={styles.tabBar}>
    <div class={styles.tabLeftSection}>
        <div class={styles.tabsContainer}>
            {#each Object.keys(app.instancesMap) as symbol (symbol)}
                <button
                    class="{styles.tabBtn} {symbol === app.activeTab ? styles.tabActive : ''}"
                    onclick={() => selectTab(symbol)}
                >
                    <span class={styles.tabLabel}>[{symbol}]</span>
                    <!-- svelte-ignore a11y_no_static_element_interactions -->
                    <span class={styles.tabClose} role="button" tabindex="0" onkeydown={(e: KeyboardEvent) => { if (e.key === 'Enter') removeInstance(symbol); }} onclick={(e: MouseEvent) => { e.stopPropagation(); removeInstance(symbol); }}>&times;</span>
                </button>
            {/each}

            {#if showAddInput}
                <div class={styles.addPairField}>
                    <input
                        type="text"
                        class={styles.pairInput}
                        placeholder="SYMBOL"
                        maxlength="10"
                        bind:value={newPairInput}
                        onkeydown={(e: KeyboardEvent) => { if (e.key === 'Enter') confirmAdd(); if (e.key === 'Escape') { showAddInput = false; newPairInput = ''; } }}
                    />
                    <button class={styles.addConfirmBtn} onclick={confirmAdd}>+</button>
                    <button class={styles.addCancelBtn} onclick={() => { showAddInput = false; newPairInput = ''; }}>&times;</button>
                </div>
            {:else}
                <button class="{styles.tabBtn} {styles.addTabBtn}" onclick={() => showAddInput = true}>[ + Add Instance ]</button>
            {/if}
        </div>
    </div>

    <div class="{styles.statusBadge} {app.isConnected ? styles.statusOnline : styles.statusOffline}">
        <span class="{styles.statusPulseDot} {app.isConnected ? styles.dotOnline : styles.dotOffline} animate-pulse"></span>
        <span>{app.isConnected ? 'LIVE' : 'OFFLINE'}</span>
    </div>
</div>
