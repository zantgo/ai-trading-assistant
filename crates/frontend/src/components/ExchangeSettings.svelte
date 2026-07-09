<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import type { ExchangeAccount } from '../types';
    import styles from './ExchangeSettings.module.css';

    const app = useAppStore();

    $effect(() => {
        app.fetchExchangeKeys();
    });

    const EXCHANGES = ['Hyperliquid'];
    let form = app.exchangeFormDraft;
    let showForm = $state(false);

    async function handleAdd() {
        if (!form.account_name.trim() || !form.api_key.trim() || !form.api_secret.trim()) return;
        await app.addExchangeKey();
        showForm = false;
    }

    async function handleDelete(id: number) {
        await app.deleteExchangeKey(id);
    }

    function formatTs(ts: number | null): string {
        if (!ts) return '--';
        return new Date(ts * 1000).toLocaleString();
    }
</script>

<div class={styles.esLayout}>
    <div class={styles.esMain}>
        <!-- Add Account Form -->
        <div class={styles.esCard}>
            <div class={styles.esCardHeader}>
                <h3 class={styles.esCardTitle}>LINK NEW EXCHANGE ACCOUNT</h3>
                <button class={styles.esToggleFormBtn} onclick={() => showForm = !showForm}>
                    {showForm ? 'Cancel' : '+ Add Account'}
                </button>
            </div>

            {#if showForm}
                <div class={styles.esForm}>
                    <div class={styles.esFieldRow}>
                        <label class={styles.esLabel} for="es-exchange">Exchange</label>
                        <select id="es-exchange" class={styles.esSelect} bind:value={form.exchange}>
                            {#each EXCHANGES as ex}
                                <option value={ex}>{ex}</option>
                            {/each}
                        </select>
                    </div>
                    <div class={styles.esFieldRow}>
                        <label class={styles.esLabel} for="es-account">Account Name</label>
                        <input id="es-account" type="text" class={styles.esInput} bind:value={form.account_name} placeholder="My Account" />
                    </div>
                    <div class={styles.esFieldRow}>
                        <label class={styles.esLabel} for="es-apikey">API Key</label>
                        <input id="es-apikey" type="password" class={styles.esInput} bind:value={form.api_key} placeholder="sk-..." />
                    </div>
                    <div class={styles.esFieldRow}>
                        <label class={styles.esLabel} for="es-apisecret">API Secret</label>
                        <input id="es-apisecret" type="password" class={styles.esInput} bind:value={form.api_secret} placeholder="••••••••" />
                    </div>
                    <div class={styles.esFieldRow}>
                        <label class={styles.esLabel} for="es-passphrase">Passphrase</label>
                        <input id="es-passphrase" type="password" class={styles.esInput} bind:value={form.passphrase} placeholder="Optional" />
                    </div>
                    <div class={styles.esFieldRow}>
                        <label class={styles.esLabel} for="es-referred">Referred UID</label>
                        <input id="es-referred" type="text" class={styles.esInput} bind:value={form.referred_uid} placeholder="Optional" />
                    </div>
                    <div class={styles.esFieldRow}>
                        <label class={styles.esLabel} for="es-active">Active Account</label>
                        <input id="es-active" type="checkbox" bind:checked={form.is_active} class={styles.esCheckbox} />
                    </div>
                    <button class={styles.esSubmitBtn} onclick={handleAdd}
                        disabled={!form.account_name.trim() || !form.api_key.trim() || !form.api_secret.trim()}>
                        ADD ACCOUNT
                    </button>
                </div>
            {/if}
        </div>

        <!-- Linked Accounts Table -->
        <div class={styles.esCard}>
            <div class={styles.esCardHeader}>
                <h3 class={styles.esCardTitle}>LINKED ACCOUNTS</h3>
                <span class={styles.esCounter}>Active Accounts ({app.exchangeActiveCount}/{app.exchangeMaxAccounts})</span>
            </div>

            {#if app.exchangeAccounts.length === 0}
                <p class={styles.esEmpty}>No linked accounts found. Add an account to get started.</p>
            {:else}
                <div class={styles.esTableWrap}>
                    <table class={styles.esTable}>
                        <thead>
                            <tr>
                                <th>Exchange</th>
                                <th>Account Name</th>
                                <th>API Key</th>
                                <th>Passphrase</th>
                                <th>Referred UID</th>
                                <th>Status</th>
                                <th>Last Sync</th>
                                <th>Action</th>
                            </tr>
                        </thead>
                        <tbody>
                            {#each app.exchangeAccounts as account (account.id)}
                                <tr>
                                    <td>{account.exchange}</td>
                                    <td>{account.account_name}</td>
                                    <td class={styles.esMonospace}>{account.api_key.substring(0, 12)}...</td>
                                    <td>••••</td>
                                    <td>{account.referred_uid || '--'}</td>
                                    <td>
                                        <span class="{styles.esStatus} {account.is_active ? styles.esActive : styles.esInactive}">
                                            {account.is_active ? 'Active' : 'Inactive'}
                                        </span>
                                    </td>
                                    <td class={styles.esTs}>{formatTs(account.last_sync_timestamp)}</td>
                                    <td>
                                        <button class={styles.esDeleteBtn} onclick={() => handleDelete(account.id)}>Delete</button>
                                    </td>
                                </tr>
                            {/each}
                        </tbody>
                    </table>
                </div>
            {/if}

            {#if app.exchangeAccounts.length > 0}
                <div class={styles.esMultiBadge}>Multi-account active</div>
            {/if}
        </div>
    </div>
</div>

