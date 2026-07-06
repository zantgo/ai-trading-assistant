<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import { createInstance } from '../lib/api.svelte';
    import styles from './TabHeader.module.css';
    const app = useAppStore();

    let newPairInput = $state('');
    let showAddInput = $state(false);
    let addError = $state<string | null>(null);
    let addLoading = $state(false);

    function selectTab(pairKey: string) {
        app.activeTab = pairKey;
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
                        oninput={() => { if (addError) addError = null; }}
                        onkeydown={(e: KeyboardEvent) => { if (e.key === 'Enter') confirmAdd(); if (e.key === 'Escape') cancelAdd(); }}
                    />
                    <span class={styles.pairQuote} title="Settlement currency is set by your session">-{app.quote}</span>
                    <button class={styles.addConfirmBtn} onclick={confirmAdd} disabled={addLoading}>{addLoading ? '…' : '+'}</button>
                    <button class={styles.addCancelBtn} onclick={cancelAdd}>&times;</button>
                    {#if addError}
                        <span class={styles.addPairError} role="alert">⚠ {addError}</span>
                    {/if}
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
