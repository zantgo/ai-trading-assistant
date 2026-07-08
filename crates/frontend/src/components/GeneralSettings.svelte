<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import Icon from '../lib/Icon.svelte';
    import styles from './GeneralSettings.module.css';

    const app = useAppStore();

    // Safety settings
    let draftLossCaution = $state(3);
    let draftLossDropout = $state(5);
    let draftDropoutHours = $state(8);
    let draftDrawdownPct = $state(30);

    // API failover settings
    let draftFailoverRetries = $state(5);
    let draftFailoverDelay = $state(30);
    let draftFailoverMax = $state(10);

    // Backup API key
    let draftBackupKey = $state('');
    let backupKeyStatus = $state<'idle' | 'saving' | 'success' | 'error'>('idle');

    // Prompts
    let draftOrchestratorPrompt = $state('');
    let draftTrendAgentPrompt = $state('');
    let promptsStatus = $state<'idle' | 'loading' | 'saving' | 'success' | 'error'>('idle');

    // Instance Limits
    let draftMaxInstances = $state(100);
    let maxInstancesStatus = $state<'idle' | 'saving' | 'success' | 'error'>('idle');

    let loaded = $state(false);

    async function loadSettings() {
        try {
            const res = await fetch('/api/config');
            const config = await res.json();

            if (config.safety) {
                draftLossCaution = config.safety.consecutive_loss_caution ?? 3;
                draftLossDropout = config.safety.consecutive_loss_dropout ?? 5;
                draftDropoutHours = config.safety.dropout_duration_hours ?? 8;
                draftDrawdownPct = config.safety.capital_drawdown_pct ?? 30;
            }
            if (config.api_failover) {
                draftFailoverRetries = config.api_failover.max_retries_per_call ?? 5;
                draftFailoverDelay = config.api_failover.retry_delay_seconds ?? 30;
                draftFailoverMax = config.api_failover.max_consecutive_failures ?? 10;
            }
            if (config.workspace?.backup_api_key) {
                draftBackupKey = config.workspace.backup_api_key;
            }
            if (config.workspace?.max_instances) {
                draftMaxInstances = config.workspace.max_instances;
            }
            loaded = true;
        } catch (e) {
            console.error('Failed to load settings:', e);
        }
    }

    async function saveSafety() {
        try {
            const res = await fetch('/api/config');
            const config = await res.json();
            config.safety = {
                consecutive_loss_caution: Number(draftLossCaution),
                consecutive_loss_dropout: Number(draftLossDropout),
                dropout_duration_hours: Number(draftDropoutHours),
                capital_drawdown_pct: Number(draftDrawdownPct),
            };
            config.api_failover = {
                max_retries_per_call: Number(draftFailoverRetries),
                retry_delay_seconds: Number(draftFailoverDelay),
                max_consecutive_failures: Number(draftFailoverMax),
            };
            await fetch('/api/config', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(config),
            });
        } catch (_) {}
    }

    async function saveBackupKey() {
        backupKeyStatus = 'saving';
        try {
            const res = await fetch('/api/settings/backup-api-key', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ api_key: draftBackupKey.trim() }),
            });
            backupKeyStatus = res.ok ? 'success' : 'error';
            if (res.ok) setTimeout(() => { backupKeyStatus = 'idle'; }, 2000);
        } catch (_) {
            backupKeyStatus = 'error';
        }
    }

    async function loadPrompts() {
        promptsStatus = 'loading';
        try {
            const [rulesRes, promptsRes] = await Promise.all([
                fetch('/api/rules'),
                fetch('/api/config'),
            ]);
            if (rulesRes.ok) {
                const data = await rulesRes.json();
                draftOrchestratorPrompt = data.content || '';
                draftTrendAgentPrompt = '';
            }
            promptsStatus = 'idle';
        } catch (_) {
            promptsStatus = 'error';
        }
    }

    async function savePrompts() {
        promptsStatus = 'saving';
        try {
            await fetch('/api/rules', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ content: draftOrchestratorPrompt }),
            });
            promptsStatus = 'success';
            setTimeout(() => { promptsStatus = 'idle'; }, 2000);
        } catch (_) {
            promptsStatus = 'error';
        }
    }

    async function saveMaxInstances() {
        maxInstancesStatus = 'saving';
        try {
            const res = await fetch('/api/settings/max-instances', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ max_instances: Number(draftMaxInstances) }),
            });
            maxInstancesStatus = res.ok ? 'success' : 'error';
            if (res.ok) {
                app.sessionMaxInstances = Number(draftMaxInstances);
                setTimeout(() => { maxInstancesStatus = 'idle'; }, 2000);
            }
        } catch (_) {
            maxInstancesStatus = 'error';
        }
    }

    const perInstance = $derived(
        app.sessionCapital && draftMaxInstances > 0
            ? app.sessionCapital / draftMaxInstances
            : 0
    );

    $effect(() => { loadSettings(); loadPrompts(); });
</script>

<div class={styles.settingsView}>
    <h2>General Settings</h2>

    {#if !loaded}
        <div class={styles.loadingRow}>Loading settings...</div>
    {:else}
        <div class={styles.settingsGrid}>
            <!-- Safety Dropdowns -->
            <div class={styles.settingsCard}>
                <h3><Icon name="shield" size={15} /> Safety Dropdowns</h3>
                <div class={styles.inputRow}>
                    <label for="loss-caution">Consecutive Loss Caution:</label>
                    <input id="loss-caution" type="number" bind:value={draftLossCaution} min="1" max="100" />
                    <span class={styles.hint}>≥ this → AI becomes cautious</span>
                </div>
                <div class={styles.inputRow}>
                    <label for="loss-dropout">Consecutive Loss Dropout:</label>
                    <input id="loss-dropout" type="number" bind:value={draftLossDropout} min="1" max="100" />
                    <span class={styles.hint}>≥ this → instance suspended</span>
                </div>
                <div class={styles.inputRow}>
                    <label for="dropout-hours">Dropout Duration (hours):</label>
                    <input id="dropout-hours" type="number" bind:value={draftDropoutHours} min="1" max="168" />
                </div>
                <div class={styles.inputRow}>
                    <label for="drawdown-pct">Capital Drawdown Limit (%):</label>
                    <input id="drawdown-pct" type="number" bind:value={draftDrawdownPct} min="1" max="100" step="0.5" />
                    <span class={styles.hint}>% loss from initial capital → stop</span>
                </div>
                <button class={styles.saveBtn} onclick={saveSafety}>Save Safety Settings</button>
            </div>

            <!-- API Failover -->
            <div class={styles.settingsCard}>
                <h3><Icon name="refresh" size={15} /> API Failover</h3>
                <div class={styles.inputRow}>
                    <label for="failover-retries">Max Retries Per Call:</label>
                    <input id="failover-retries" type="number" bind:value={draftFailoverRetries} min="1" max="20" />
                </div>
                <div class={styles.inputRow}>
                    <label for="failover-delay">Retry Delay (seconds):</label>
                    <input id="failover-delay" type="number" bind:value={draftFailoverDelay} min="1" max="300" />
                </div>
                <div class={styles.inputRow}>
                    <label for="failover-max">Max Consecutive Failures:</label>
                    <input id="failover-max" type="number" bind:value={draftFailoverMax} min="1" max="50" />
                    <span class={styles.hint}>halt instance after this many</span>
                </div>
            </div>

            <!-- Backup API Key -->
            <div class={styles.settingsCard}>
                <h3>🔑 Backup API Key</h3>
                <div class={styles.inputRow}>
                    <label for="backup-key">Global Backup Key:</label>
                    <input id="backup-key" type="password" bind:value={draftBackupKey} placeholder="sk-..." />
                </div>
                <button class={styles.saveBtn} onclick={saveBackupKey} disabled={backupKeyStatus === 'saving'}>
                    {backupKeyStatus === 'saving' ? 'Saving...' : backupKeyStatus === 'success' ? '✓ Saved' : 'Save Backup Key'}
                </button>
            </div>

            <!-- Instance Limits -->
            <div class={styles.settingsCard}>
                <h3><Icon name="list" size={15} /> Instance Limits</h3>
                <div class={styles.inputRow}>
                    <label for="max-instances">Max Instances:</label>
                    <input id="max-instances" type="number" bind:value={draftMaxInstances} min="1" max="1000" />
                </div>
                <div class={styles.inputRow}>
                    <span class={styles.readonlyLabel}>Per-Instance Capital:</span>
                    <span class={styles.capitalDisplay}>
                        {app.sessionCurrency || 'USDT'} {perInstance.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
                    </span>
                </div>
                <span class={styles.hint}>Portfolio of {app.sessionCurrency || 'USDT'} {app.sessionCapital?.toLocaleString() || '0'} divided into {draftMaxInstances} equal parts</span>
                <button class={styles.saveBtn} onclick={saveMaxInstances} disabled={maxInstancesStatus === 'saving'}>
                    {maxInstancesStatus === 'saving' ? 'Saving...' : maxInstancesStatus === 'success' ? '✓ Saved' : 'Save Instance Limits'}
                </button>
            </div>

            <!-- System Prompts -->
            <div class="{styles.settingsCard} {styles.fullWidth}">
                <h3>📝 System Prompts</h3>
                <div class={styles.inputRow}>
                    <label for="orchestrator-prompt">Orchestrator / Rules Guide:</label>
                </div>
                <textarea id="orchestrator-prompt" bind:value={draftOrchestratorPrompt} rows="12" class={styles.promptEditor}></textarea>
                <button class={styles.saveBtn} onclick={savePrompts} disabled={promptsStatus === 'saving'}>
                    {promptsStatus === 'saving' ? 'Saving...' : promptsStatus === 'success' ? '✓ Saved' : 'Save Prompts'}
                </button>
                <span class={styles.hint}>Edits the indicators-guide.md that all agents and the orchestrator reference.</span>
            </div>
        </div>
    {/if}
</div>

