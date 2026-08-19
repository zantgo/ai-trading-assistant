<script lang="ts">
    import { useAppStore } from './state.svelte';
    import { createInstance, postInstanceConfig } from './lib/api.svelte';
    import styles from './LaunchSetup.module.css';

    const app = useAppStore();

    type LaunchMode = 'observe' | 'paper' | 'live';

    interface DraftInstance {
        base: string;
        micro: number;
        fast: number;
        slow: number;
        macro: number;
    }

    // v7.2 parity: the default ladder is the registry's ladder
    // (`registry::add_instance` fallback): micro 60s, fast 180s, slow/macro
    // from the workspace config (via /api/config). GUI, CLI, and registry
    // derive their defaults from the same source.
    function tfDefaults() {
        return {
            micro: 60,
            fast: 180,
            slow: app.workspaceSlowTimeframeSecs > 0 ? app.workspaceSlowTimeframeSecs : 300,
            macro: app.workspaceMacroTimeframeSecs > 0 ? app.workspaceMacroTimeframeSecs : 900,
        };
    }

    const MODE_META: Record<LaunchMode, { title: string; verb: string; badge: string; description: string }> = {
        observe: {
            title: 'Observe',
            verb: 'Monitor',
            badge: 'No orders',
            description: 'Monitor markets and signals without executing trades. The safest mode.',
        },
        paper: {
            title: 'Simulate',
            verb: 'Paper',
            badge: 'Simulated',
            description: 'Execute simulated orders with paper capital. Full strategy execution.',
        },
        live: {
            title: 'Execute',
            verb: 'Live',
            badge: 'Real orders',
            description: 'Execute real trades with real capital. Production environment.',
        },
    };

    const MODE_ORDER: LaunchMode[] = ['observe', 'paper', 'live'];

    // ─── Wizard state ────────────────────────────────────────────────
    let step = $state(1);
    let mode = $state<LaunchMode>('observe');
    let exchange = $state('Hyperliquid');
    let currency = $state('USDC');
    let capital = $state(1000);
    let walletAddress = $state('');
    let privateKey = $state('');
    let apiKey = $state('');
    let apiSecret = $state('');
    let passphrase = $state('');
    let instances = $state<DraftInstance[]>([]);
    let newBase = $state('');
    let newTfs = $state(tfDefaults());
    let error = $state<string | null>(null);
    let loading = $state(false);

    // Perpetual-futures settlement rules per exchange:
    //  - Hyperliquid settles exclusively in USDC.
    //  - Bitget's dashboard exposes only USDT-M futures.
    const supportedCurrencies = $derived(
        exchange === 'Hyperliquid' ? ['USDC'] : ['USDT']
    );

    $effect(() => {
        if (!supportedCurrencies.includes(currency)) {
            currency = supportedCurrencies[0];
        }
    });

    function currencyAvailable(c: string): boolean {
        return supportedCurrencies.includes(c);
    }

    const stepTitles = ['Mode', 'Environment', 'Instances', 'Review'];

    function goNext() {
        error = null;
        if (step < 4) step += 1;
    }

    function goBack() {
        error = null;
        if (step > 1) step -= 1;
    }

    function selectMode(m: LaunchMode) {
        mode = m;
        error = null;
    }

    function addInstance() {
        const base = newBase.trim().toUpperCase();
        if (!/^[A-Z0-9]{2,10}$/.test(base)) {
            error = 'Invalid ticker. Must be 2-10 alphanumeric characters.';
            return;
        }
        if (instances.some((i) => i.base === base)) {
            error = `${base} is already in the instance list.`;
            return;
        }
        instances = [
            ...instances,
            {
                base,
                micro: clampTf(newTfs.micro),
                fast: clampTf(newTfs.fast),
                slow: clampTf(newTfs.slow),
                macro: clampTf(newTfs.macro),
            },
        ];
        newBase = '';
        newTfs = tfDefaults();
        error = null;
    }

    function removeInstance(index: number) {
        instances = instances.filter((_, i) => i !== index);
    }

    function clampTf(v: number): number {
        const n = Number(v);
        if (!Number.isFinite(n)) return 60;
        return Math.min(86400, Math.max(10, Math.round(n)));
    }

    function tfLabel(secs: number): string {
        if (secs % 3600 === 0) return `${secs / 3600}h`;
        if (secs % 60 === 0) return `${secs / 60}m`;
        return `${secs}s`;
    }

    async function readBackendError(res: Response, fallback: string): Promise<string> {
        try {
            const ct = res.headers.get('content-type') || '';
            if (ct.includes('application/json')) {
                const data = await res.json();
                return (data && (data.error || data.message)) || fallback;
            }
            const text = await res.text();
            return text.trim() || fallback;
        } catch {
            return fallback;
        }
    }

    async function handleLaunch() {
        error = null;
        loading = true;
        try {
            // 1. Execute mode: persist exchange credentials (encrypted server-side).
            if (mode === 'live') {
                const keyBody = exchange === 'Hyperliquid'
                    ? {
                        exchange,
                        account_name: 'launch-setup',
                        api_key: walletAddress.trim(),
                        api_secret: privateKey.trim(),
                        is_active: true,
                    }
                    : {
                        exchange,
                        account_name: 'launch-setup',
                        api_key: apiKey.trim(),
                        api_secret: apiSecret.trim(),
                        passphrase: passphrase.trim(),
                        is_active: true,
                    };
                const keyRes = await fetch('/api/keys', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(keyBody),
                });
                if (!keyRes.ok) {
                    throw new Error(await readBackendError(keyRes, 'Failed to save exchange credentials.'));
                }
            }

            // 2. Initialize the session (mode becomes the default for created instances).
            const init = await app.initSession(
                currency,
                exchange,
                mode,
                mode === 'paper' ? Number(capital) : undefined,
            );
            if (!init.success) {
                throw new Error(init.error || 'Failed to initialize session.');
            }

            // 3. Create each staged instance with its timeframe configuration.
            for (const draft of instances) {
                const created = await createInstance(draft.base, app.quote);
                if (!created.ok) {
                    throw new Error(created.error || `Failed to add ${draft.base}.`);
                }
                const instanceId = created.instanceId ?? draft.base;
                const ok = await postInstanceConfig(instanceId, {
                    micro_term: { candles: { duration_seconds: draft.micro } },
                    fast_term: { candles: { duration_seconds: draft.fast } },
                    slow_term: { candles: { duration_seconds: draft.slow } },
                    macro_term: { candles: { duration_seconds: draft.macro } },
                });
                if (!ok) {
                    throw new Error(`Instance ${draft.base} was created but its timeframe configuration failed.`);
                }
                app.initInstance(draft.base, exchange, created.instanceId);
            }

            // 4. Land on the workspace with the first instance selected.
            const firstKey = instances.length > 0 ? app.pairKeyFor(instances[0].base) : null;
            if (firstKey) {
                app.enterInstance(firstKey);
            } else {
                app.currentEngine = 'market_monitor';
                app.middleTab = 'overview';
                app.activeEngineTab = 'overview';
                app.selectedInstance = null;
            }
        } catch (e: any) {
            error = e?.message || 'Launch failed.';
        }
        loading = false;
    }
</script>

<div class={styles.launchGate}>
    <div class={styles.launchCard}>
        <header class={styles.launchHeader}>
            <h1 class={styles.launchTitle}>Trading Platform</h1>
            <p class={styles.launchSubtitle}>Launch Setup — choose how you want to start</p>
            <nav class={styles.steps} aria-label="Setup steps">
                {#each stepTitles as title, i (title)}
                    <span class="{styles.step} {i + 1 === step ? styles.stepActive : ''} {i + 1 < step ? styles.stepDone : ''}">
                        <span class={styles.stepDot}>{i + 1 < step ? '✓' : i + 1}</span>
                        {title}
                    </span>
                {/each}
            </nav>
        </header>

        {#if step === 1}
            <section class={styles.section}>
                <h2 class={styles.sectionTitle}>Choose how you want to start</h2>
                <div class={styles.modeCards}>
                    {#each MODE_ORDER as m, i (m)}
                        <button
                            class="{styles.modeCard} {mode === m ? styles.modeCardActive : ''}"
                            class:observeCard={m === 'observe'}
                            class:simulateCard={m === 'paper'}
                            class:executeCard={m === 'live'}
                            onclick={() => selectMode(m)}
                        >
                            <span class={styles.modeTitle}>{MODE_META[m].title}</span>
                            <span class={styles.modeVerb}>{MODE_META[m].verb}</span>
                            <span class={styles.modeBadge}>{MODE_META[m].badge}</span>
                            <span class={styles.modeDesc}>{MODE_META[m].description}</span>
                            <span class={styles.modeArrow}>{mode === m ? '●' : '○'}</span>
                            <span class={styles.modeStep}>{i + 1}</span>
                        </button>
                    {/each}
                </div>
            </section>
        {:else if step === 2}
            <section class={styles.section}>
                <h2 class={styles.sectionTitle}>Environment — {MODE_META[mode].title}</h2>

                <div class={styles.formGroup}>
                    <label class={styles.formLabel} for="launch-exchange">Exchange</label>
                    <select id="launch-exchange" class={styles.formSelect} bind:value={exchange}>
                        <option value="Hyperliquid">Hyperliquid</option>
                        <option value="Bitget">Bitget</option>
                    </select>
                </div>

                <div class={styles.formGroup}>
                    <span class={styles.formLabel}>Settlement Currency</span>
                    <div class={styles.radioGroup}>
                        <label class="{styles.radioOption} {!currencyAvailable('USDT') ? styles.disabled : ''} {currency === 'USDT' ? styles.active : ''}">
                            <input type="radio" name="currency" value="USDT" bind:group={currency} disabled={!currencyAvailable('USDT')} />
                            <span class={styles.radioLabel}>USDT</span>
                            <span class="{styles.radioBadge} {currencyAvailable('USDT') ? styles.enabled : styles.disabled}">
                                {currencyAvailable('USDT') ? 'Available' : 'Not available'}
                            </span>
                        </label>
                        <label class="{styles.radioOption} {!currencyAvailable('USDC') ? styles.disabled : ''} {currency === 'USDC' ? styles.active : ''}">
                            <input type="radio" name="currency" value="USDC" bind:group={currency} disabled={!currencyAvailable('USDC')} />
                            <span class={styles.radioLabel}>USDC</span>
                            <span class="{styles.radioBadge} {currencyAvailable('USDC') ? styles.enabled : styles.disabled}">
                                {currencyAvailable('USDC') ? 'Available' : 'Not available'}
                            </span>
                        </label>
                    </div>
                </div>

                {#if mode === 'paper'}
                    <div class={styles.formGroup}>
                        <label class={styles.formLabel} for="launch-capital">Starting Capital (USD)</label>
                        <input id="launch-capital" type="number" min="100" step="100"
                            class={styles.formInput} bind:value={capital} />
                        <p class={styles.formHint}>The paper balance for instances created in this session.</p>
                    </div>
                {:else if mode === 'observe'}
                    <p class={styles.formHint}>
                        Observe mode needs no capital and no credentials — instances monitor markets and
                        signals without executing trades.
                    </p>
                {:else}
                    <div class={styles.formGroup}>
                        <span class={styles.formLabel}>Exchange Credentials</span>
                        {#if exchange === 'Hyperliquid'}
                            <label class={styles.formLabel} for="launch-wallet">Wallet Address</label>
                            <input id="launch-wallet" type="text" autocomplete="off"
                                class={styles.formInput} bind:value={walletAddress}
                                placeholder="0x…" />
                            <label class={styles.formLabel} for="launch-private-key">Private Key</label>
                            <input id="launch-private-key" type="password" autocomplete="off"
                                class={styles.formInput} bind:value={privateKey}
                                placeholder="••••••••••••••••••••••••" />
                            <p class={styles.formHint}>
                                Stored encrypted (AES-256-GCM under EXCHANGE_SECRET_KEY). Live trading uses
                                the balance of your Hyperliquid account — there is no paper balance.
                            </p>
                        {:else}
                            <label class={styles.formLabel} for="launch-api-key">API Key</label>
                            <input id="launch-api-key" type="text" autocomplete="off"
                                class={styles.formInput} bind:value={apiKey} />
                            <label class={styles.formLabel} for="launch-api-secret">API Secret</label>
                            <input id="launch-api-secret" type="password" autocomplete="off"
                                class={styles.formInput} bind:value={apiSecret} />
                            <label class={styles.formLabel} for="launch-passphrase">Passphrase</label>
                            <input id="launch-passphrase" type="password" autocomplete="off"
                                class={styles.formInput} bind:value={passphrase} />
                            <p class={styles.formHint}>
                                Stored encrypted (AES-256-GCM under EXCHANGE_SECRET_KEY). Live trading uses
                                the balance of your Bitget USDT-M account.
                            </p>
                        {/if}
                    </div>
                {/if}
            </section>
        {:else if step === 3}
            <section class={styles.section}>
                <h2 class={styles.sectionTitle}>Instances</h2>
                <p class={styles.formHint}>Add one or more instances, or skip and add them later from the workspace panel.</p>

                <div class={styles.instanceList}>
                    {#each instances as inst, i (inst.base)}
                        <div class={styles.instanceRow}>
                            <span class={styles.instancePair}>{inst.base} <span class={styles.instanceQuote}>{app.quote}</span></span>
                            <span class={styles.instanceTfs}>
                                {tfLabel(inst.micro)} / {tfLabel(inst.fast)} / {tfLabel(inst.slow)} / {tfLabel(inst.macro)}
                            </span>
                            <button class={styles.removeBtn} aria-label={`Remove ${inst.base}`}
                                onclick={() => removeInstance(i)}>✕</button>
                        </div>
                    {/each}
                    {#if instances.length === 0}
                        <p class={styles.emptyHint}>No instances configured yet.</p>
                    {/if}
                </div>

                <div class={styles.addGroup}>
                    <label class={styles.formLabel} for="launch-base">Add instance</label>
                    <div class={styles.addRow}>
                        <input id="launch-base" type="text" maxlength="10"
                            class="{styles.formInput} {styles.baseInput}" bind:value={newBase}
                            placeholder="BTC" onkeydown={(e) => e.key === 'Enter' && addInstance()} />
                        {#each ['micro', 'fast', 'slow', 'macro'] as slot (slot)}
                            <label class={styles.tfField}>
                                <span class={styles.tfLabel}>{slot}</span>
                                <input type="number" min="10" max="86400" step="10"
                                    class={styles.tfInput}
                                    value={newTfs[slot as keyof typeof newTfs]}
                                    oninput={(e) => (newTfs[slot as keyof typeof newTfs] = Number((e.currentTarget as HTMLInputElement).value))} />
                            </label>
                        {/each}
                        <button class={styles.addBtn} onclick={addInstance}>+ Add</button>
                    </div>
                </div>
            </section>
        {:else}
            <section class={styles.section}>
                <h2 class={styles.sectionTitle}>Review</h2>
                <div class={styles.reviewTable}>
                    <div class={styles.reviewRow}><span class={styles.reviewKey}>Mode</span><span class={styles.reviewVal}>{MODE_META[mode].title} ({MODE_META[mode].verb})</span></div>
                    <div class={styles.reviewRow}><span class={styles.reviewKey}>Exchange</span><span class={styles.reviewVal}>{exchange}</span></div>
                    <div class={styles.reviewRow}><span class={styles.reviewKey}>Settlement Currency</span><span class={styles.reviewVal}>{currency}</span></div>
                    {#if mode === 'paper'}
                        <div class={styles.reviewRow}><span class={styles.reviewKey}>Starting Capital</span><span class={styles.reviewVal}>${Number(capital).toLocaleString()}</span></div>
                    {:else if mode === 'live'}
                        <div class={styles.reviewRow}><span class={styles.reviewKey}>Credentials</span><span class={styles.reviewVal}>
                            {exchange === 'Hyperliquid'
                                ? `wallet ${walletAddress ? '✓ set' : '✗ missing'}`
                                : `api key ${apiKey ? '✓ set' : '✗ missing'}`}
                        </span></div>
                    {/if}
                    <div class={styles.reviewRow}>
                        <span class={styles.reviewKey}>Instances</span>
                        <span class={styles.reviewVal}>
                            {#if instances.length === 0}
                                None — add later from the workspace panel
                            {:else}
                                {instances.length} configured
                            {/if}
                        </span>
                    </div>
                    {#each instances as inst (inst.base)}
                        <div class={styles.reviewRow}><span class={styles.reviewKey}></span><span class={styles.reviewVal}>
                            {inst.base}-{app.quote} · {tfLabel(inst.micro)} / {tfLabel(inst.fast)} / {tfLabel(inst.slow)} / {tfLabel(inst.macro)}
                        </span></div>
                    {/each}
                </div>
            </section>
        {/if}

        {#if error}
            <div class={styles.formError}>{error}</div>
        {/if}

        <footer class={styles.footer}>
            {#if step > 1}
                <button class={styles.backButton} onclick={goBack} disabled={loading}>Back</button>
            {:else}
                <span></span>
            {/if}
            {#if step < 4}
                <button class={styles.primaryButton} onclick={goNext} disabled={loading}>
                    Continue
                </button>
            {:else}
                <button class={styles.launchButton} onclick={handleLaunch} disabled={loading}>
                    {#if loading}
                        <span class={styles.spinner}></span>
                        Launching…
                    {:else}
                        Launch
                    {/if}
                </button>
            {/if}
        </footer>
    </div>
</div>
