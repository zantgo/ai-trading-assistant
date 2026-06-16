<script lang="ts">
    import { getState } from '../state.svelte';
    import type { ExchangeAccount } from '../state.svelte';

    const app = getState();

    $effect(() => {
        app.fetchExchangeAccounts();
    });

    const EXCHANGES = ['Bitget', 'Hyperliquid'];
    let form = app.exchangeFormDraft;
    let showForm = $state(false);

    async function handleAdd() {
        if (!form.account_name.trim() || !form.api_key.trim() || !form.api_secret.trim()) return;
        await app.addExchangeAccount();
        showForm = false;
    }

    async function handleDelete(id: number) {
        await app.deleteExchangeAccount(id);
    }

    function formatTs(ts: number | null): string {
        if (!ts) return '--';
        return new Date(ts * 1000).toLocaleString();
    }
</script>

<div class="es-layout">
    <div class="es-main">
        <!-- Add Account Form -->
        <div class="es-card">
            <div class="es-card-header">
                <h3 class="es-card-title">LINK NEW EXCHANGE ACCOUNT</h3>
                <button class="es-toggle-form-btn" onclick={() => showForm = !showForm}>
                    {showForm ? 'Cancel' : '+ Add Account'}
                </button>
            </div>

            {#if showForm}
                <div class="es-form">
                    <div class="es-field-row">
                        <label class="es-label">Exchange</label>
                        <select class="es-select" bind:value={form.exchange}>
                            {#each EXCHANGES as ex}
                                <option value={ex}>{ex}</option>
                            {/each}
                        </select>
                    </div>
                    <div class="es-field-row">
                        <label class="es-label">Account Name</label>
                        <input type="text" class="es-input" bind:value={form.account_name} placeholder="My Account" />
                    </div>
                    <div class="es-field-row">
                        <label class="es-label">API Key</label>
                        <input type="password" class="es-input" bind:value={form.api_key} placeholder="sk-..." />
                    </div>
                    <div class="es-field-row">
                        <label class="es-label">API Secret</label>
                        <input type="password" class="es-input" bind:value={form.api_secret} placeholder="••••••••" />
                    </div>
                    <div class="es-field-row">
                        <label class="es-label">Passphrase</label>
                        <input type="password" class="es-input" bind:value={form.passphrase} placeholder="Required for Bitget" />
                    </div>
                    <div class="es-field-row">
                        <label class="es-label">Referred UID</label>
                        <input type="text" class="es-input" bind:value={form.referred_uid} placeholder="Optional" />
                    </div>
                    <div class="es-field-row">
                        <label class="es-label">Active Account</label>
                        <input type="checkbox" bind:checked={form.is_active} class="es-checkbox" />
                    </div>
                    <button class="es-submit-btn" onclick={handleAdd}
                        disabled={!form.account_name.trim() || !form.api_key.trim() || !form.api_secret.trim()}>
                        ADD ACCOUNT
                    </button>
                </div>
            {/if}
        </div>

        <!-- Linked Accounts Table -->
        <div class="es-card">
            <div class="es-card-header">
                <h3 class="es-card-title">LINKED ACCOUNTS</h3>
                <span class="es-counter">Active Accounts ({app.exchangeActiveCount}/{app.exchangeMaxAccounts})</span>
            </div>

            {#if app.exchangeAccounts.length === 0}
                <p class="es-empty">No linked accounts found. Add an account to get started.</p>
            {:else}
                <div class="es-table-wrap">
                    <table class="es-table">
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
                                    <td class="es-monospace">{account.api_key.substring(0, 12)}...</td>
                                    <td>••••</td>
                                    <td>{account.referred_uid || '--'}</td>
                                    <td>
                                        <span class="es-status" class:es-active={account.is_active} class:es-inactive={!account.is_active}>
                                            {account.is_active ? 'Active' : 'Inactive'}
                                        </span>
                                    </td>
                                    <td class="es-ts">{formatTs(account.last_sync_timestamp)}</td>
                                    <td>
                                        <button class="es-delete-btn" onclick={() => handleDelete(account.id)}>Delete</button>
                                    </td>
                                </tr>
                            {/each}
                        </tbody>
                    </table>
                </div>
            {/if}

            {#if app.exchangeAccounts.length > 0}
                <div class="es-multi-badge">Multi-account active</div>
            {/if}
        </div>
    </div>
</div>

<style>
    .es-layout { max-width: 1000px; margin: 0 auto; width: 100%; padding: 16px; box-sizing: border-box; }
    .es-main { display: flex; flex-direction: column; gap: 16px; }
    .es-card { background: #131722; border: 1px solid #2a2e39; border-radius: 8px; padding: 16px; }
    .es-card-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px; }
    .es-card-title { font-size: 11px; font-weight: 700; color: #64748b; text-transform: uppercase; letter-spacing: 0.05em; margin: 0; }
    .es-counter { font-size: 10px; color: #60a5fa; font-weight: 600; }

    .es-toggle-form-btn {
        background: #1e40af; border: 1px solid #3b82f6; color: #f1f5f9; padding: 6px 14px;
        border-radius: 4px; font-size: 10px; font-weight: 700; cursor: pointer;
    }
    .es-toggle-form-btn:hover { background: #1e3a8a; }

    .es-form { display: flex; flex-direction: column; gap: 10px; }
    .es-field-row { display: flex; flex-direction: column; gap: 4px; }
    .es-label { font-size: 10px; font-weight: 600; color: #94a3b8; text-transform: uppercase; }
    .es-input {
        background: #0f131c; border: 1px solid #2a2e39; color: #e2e8f0; padding: 8px 10px;
        border-radius: 4px; font-size: 11px; outline: none;
    }
    .es-input:focus { border-color: #3b82f6; }
    .es-select {
        background: #0f131c; border: 1px solid #2a2e39; color: #e2e8f0; padding: 8px 10px;
        border-radius: 4px; font-size: 11px; outline: none; cursor: pointer;
    }
    .es-checkbox { width: 18px; height: 18px; accent-color: #3b82f6; cursor: pointer; }

    .es-submit-btn {
        width: 100%; padding: 10px; background: linear-gradient(135deg, #1e40af, #3b82f6);
        border: 1px solid #3b82f6; color: #f1f5f9; font-size: 11px; font-weight: 700; text-transform: uppercase;
        border-radius: 6px; cursor: pointer; margin-top: 6px;
    }
    .es-submit-btn:disabled { opacity: 0.5; cursor: not-allowed; }

    .es-empty { font-size: 11px; color: #64748b; text-align: center; padding: 20px; font-style: italic; }

    .es-table-wrap { overflow-x: auto; }
    .es-table {
        width: 100%; border-collapse: collapse; font-size: 10px; color: #94a3b8;
    }
    .es-table thead { position: sticky; top: 0; background: #131722; }
    .es-table th {
        text-align: left; padding: 6px 8px; font-weight: 700; color: #64748b; text-transform: uppercase;
        letter-spacing: 0.04em; border-bottom: 1px solid #1e293b; font-size: 9px;
    }
    .es-table td { padding: 6px 8px; border-bottom: 1px solid #0f131c; white-space: nowrap; }
    .es-table tbody tr:hover { background: #1a1f2e; }
    .es-monospace { font-family: monospace; font-size: 9px; }
    .es-ts { font-size: 9px; color: #64748b; }

    .es-status {
        padding: 2px 8px; border-radius: 10px; font-size: 9px; font-weight: 700; text-transform: uppercase;
    }
    .es-active { background: rgba(16,185,129,0.12); color: #10b981; }
    .es-inactive { background: rgba(100,116,139,0.12); color: #64748b; }

    .es-delete-btn {
        background: none; border: 1px solid rgba(239,68,68,0.3); color: #ef4444;
        padding: 3px 8px; border-radius: 3px; font-size: 9px; font-weight: 600; cursor: pointer;
    }
    .es-delete-btn:hover { background: rgba(239,68,68,0.12); }

    .es-multi-badge {
        margin-top: 10px; text-align: center; font-size: 9px; color: #60a5fa; font-weight: 600;
    }
</style>
