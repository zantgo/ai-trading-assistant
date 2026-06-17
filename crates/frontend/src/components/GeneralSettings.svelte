<script lang="ts">
    import { getState } from '../state.svelte';

    const app = getState();

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

    $effect(() => { loadSettings(); loadPrompts(); });
</script>

<div class="settings-view">
    <h2>General Settings</h2>

    {#if !loaded}
        <div class="loading-row">Loading settings...</div>
    {:else}
        <div class="settings-grid">
            <!-- Safety Dropdowns -->
            <div class="settings-card">
                <h3>🛡️ Safety Dropdowns</h3>
                <div class="input-row">
                    <label>Consecutive Loss Caution:</label>
                    <input type="number" bind:value={draftLossCaution} min="1" max="100" />
                    <span class="hint">≥ this → AI becomes cautious</span>
                </div>
                <div class="input-row">
                    <label>Consecutive Loss Dropout:</label>
                    <input type="number" bind:value={draftLossDropout} min="1" max="100" />
                    <span class="hint">≥ this → instance suspended</span>
                </div>
                <div class="input-row">
                    <label>Dropout Duration (hours):</label>
                    <input type="number" bind:value={draftDropoutHours} min="1" max="168" />
                </div>
                <div class="input-row">
                    <label>Capital Drawdown Limit (%):</label>
                    <input type="number" bind:value={draftDrawdownPct} min="1" max="100" step="0.5" />
                    <span class="hint">% loss from initial capital → stop</span>
                </div>
                <button class="save-btn" onclick={saveSafety}>Save Safety Settings</button>
            </div>

            <!-- API Failover -->
            <div class="settings-card">
                <h3>🔄 API Failover</h3>
                <div class="input-row">
                    <label>Max Retries Per Call:</label>
                    <input type="number" bind:value={draftFailoverRetries} min="1" max="20" />
                </div>
                <div class="input-row">
                    <label>Retry Delay (seconds):</label>
                    <input type="number" bind:value={draftFailoverDelay} min="1" max="300" />
                </div>
                <div class="input-row">
                    <label>Max Consecutive Failures:</label>
                    <input type="number" bind:value={draftFailoverMax} min="1" max="50" />
                    <span class="hint">halt instance after this many</span>
                </div>
            </div>

            <!-- Backup API Key -->
            <div class="settings-card">
                <h3>🔑 Backup API Key</h3>
                <div class="input-row">
                    <label>Global Backup Key:</label>
                    <input type="password" bind:value={draftBackupKey} placeholder="sk-..." />
                </div>
                <button class="save-btn" onclick={saveBackupKey} disabled={backupKeyStatus === 'saving'}>
                    {backupKeyStatus === 'saving' ? 'Saving...' : backupKeyStatus === 'success' ? '✓ Saved' : 'Save Backup Key'}
                </button>
            </div>

            <!-- System Prompts -->
            <div class="settings-card full-width">
                <h3>📝 System Prompts</h3>
                <div class="input-row">
                    <label>Orchestrator / Rules Guide:</label>
                </div>
                <textarea bind:value={draftOrchestratorPrompt} rows="12" class="prompt-editor"></textarea>
                <button class="save-btn" onclick={savePrompts} disabled={promptsStatus === 'saving'}>
                    {promptsStatus === 'saving' ? 'Saving...' : promptsStatus === 'success' ? '✓ Saved' : 'Save Prompts'}
                </button>
                <span class="hint">Edits the indicators-guide.md that all agents and the orchestrator reference.</span>
            </div>
        </div>
    {/if}
</div>

<style>
    .settings-view {
        padding: 1.5rem;
        color: #cbd5e1;
        max-width: 900px;
        margin: 0 auto;
    }
    .settings-view h2 { margin: 0 0 1rem 0; color: #e0e0ff; font-size: 1.2rem; }
    .settings-grid {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 1rem;
    }
    .full-width { grid-column: span 2; }
    .settings-card {
        background: #14142a;
        border: 1px solid #2a2a4a;
        border-radius: 8px;
        padding: 1rem;
    }
    .settings-card h3 { margin: 0 0 0.75rem 0; font-size: 0.9rem; color: #e0e0ff; }
    .input-row {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        margin-bottom: 0.5rem;
        flex-wrap: wrap;
    }
    .input-row label {
        font-size: 0.78rem;
        color: #8888aa;
        min-width: 160px;
    }
    .input-row input {
        width: 100px;
        padding: 0.35rem 0.5rem;
        background: #1e1e3a;
        border: 1px solid #333355;
        border-radius: 4px;
        color: #e0e0ff;
        font-size: 0.82rem;
        outline: none;
    }
    .input-row input:focus { border-color: #5b7fff; }
    .hint {
        font-size: 0.68rem;
        color: #556;
        width: 100%;
    }
    .prompt-editor {
        width: 100%;
        margin-top: 0.5rem;
        padding: 0.6rem;
        background: #1e1e3a;
        border: 1px solid #333355;
        border-radius: 4px;
        color: #e0e0ff;
        font-family: monospace;
        font-size: 0.75rem;
        resize: vertical;
        outline: none;
        box-sizing: border-box;
    }
    .prompt-editor:focus { border-color: #5b7fff; }
    .save-btn {
        margin-top: 0.75rem;
        padding: 0.45rem 1rem;
        background: #5b7fff;
        border: none;
        border-radius: 6px;
        color: white;
        font-size: 0.82rem;
        font-weight: 600;
        cursor: pointer;
    }
    .save-btn:disabled { opacity: 0.5; cursor: not-allowed; }
    .save-btn:hover:not(:disabled) { background: #4a6eef; }
    .loading-row { text-align: center; padding: 2rem; color: #64748b; }
</style>
