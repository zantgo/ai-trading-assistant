<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import {
        createInstance,
        deleteInstanceById,
        waitForAdvisory,
    } from '../lib/api.svelte';
    import {
        connectWsForInstance,
        type WsState,
    } from '../lib/websocket.svelte';
    import {
        detectBackendErrorKind,
        decide,
        parseSymbols,
        reasonFor,
        reasonLabel,
        summarize,
        type PairOutcome,
    } from '../lib/watchlistScanner';
    import styles from './WatchlistScannerModal.module.css';
    import brutalistStyles from '../styles/brutalist-grid.module.css';

    interface Props {
        isOpen: boolean;
        wssMap: Record<string, WsState>;
        onclose: () => void;
    }

    let { isOpen, wssMap, onclose }: Props = $props();

    const app = useAppStore();

    type Phase = 'input' | 'running' | 'done';
    let phase = $state<Phase>('input');
    let inputText = $state('');
    let parsed = $derived(parseSymbols(inputText));
    let cancelRun = $state(false);

    /// Per-pair outcome table. Seeded once on phase transition to `running`
    /// and mutated in place as each pair advances through its lifecycle.
    let outcomes = $state<PairOutcome[]>([]);
    let currentIndex = $state(0);

    const sessionReady = $derived(app.sessionActive);
    const summary = $derived(phase === 'done' ? summarize(outcomes) : null);

    function reset() {
        phase = 'input';
        inputText = '';
        outcomes = [];
        currentIndex = 0;
        cancelRun = false;
    }

    function close() {
        reset();
        onclose();
    }

    function cancel() {
        cancelRun = true;
        close();
    }

    async function startRun() {
        if (parsed.length === 0) return;
        cancelRun = false;
        outcomes = parsed.map((base) => ({
            base,
            pairKey: app.pairKeyFor(base),
            status: 'pending' as const,
        }));
        currentIndex = 0;
        phase = 'running';

        for (let i = 0; i < outcomes.length; i++) {
            if (cancelRun) break;
            currentIndex = i;
            await processOne(i);
        }

        currentIndex = outcomes.length;
        phase = 'done';
    }

    async function processOne(index: number) {
        const outcome = outcomes[index];
        if (!outcome) return;
        const startedAt = Date.now();

        // 1. ADD
        outcomes[index] = { ...outcome, status: 'adding' };
        const result = await createInstance(outcome.base, app.quote);
        if (!result.ok || !result.instanceId) {
            const reason = detectBackendErrorKind(result.error);
            outcomes[index] = {
                ...outcome,
                status: 'done',
                reason,
                error: result.error,
                elapsedMs: Date.now() - startedAt,
            };
            return;
        }

        const instanceId = result.instanceId;
        app.initInstance(outcome.base, undefined, instanceId);
        if (app.instancesMap[outcome.pairKey]) {
            app.instancesMap[outcome.pairKey].instanceId = instanceId;
        }

        // 2. WIRE WS
        connectWsForInstance(app, wssMap, outcome.pairKey);

        // 3. WAIT FOR DECISION (30s cap)
        outcomes[index] = { ...outcome, status: 'waiting' };
        const verdict = await waitForAdvisory(app, outcome.pairKey, 30_000);

        if (verdict.status === 'TIMEOUT') {
            await deleteInstanceById(instanceId);
            app.removeInstance(outcome.pairKey);
            outcomes[index] = {
                ...outcome,
                status: 'done',
                reason: 'TIMEOUT',
                elapsedMs: Date.now() - startedAt,
            };
            return;
        }

        const keep = decide(verdict.decisionContext, verdict.advisory) === 'KEEP';
        if (!keep) {
            await deleteInstanceById(instanceId);
            app.removeInstance(outcome.pairKey);
            outcomes[index] = {
                ...outcome,
                status: 'done',
                reason: reasonFor('DELETE', verdict.decisionContext, verdict.advisory),
                guidance: verdict.advisory?.directional_guidance,
                tradeReadiness: verdict.decisionContext.trade_readiness,
                elapsedMs: Date.now() - startedAt,
            };
            return;
        }

        outcomes[index] = {
            ...outcome,
            status: 'done',
            reason: 'KEEP',
            guidance: verdict.advisory?.directional_guidance,
            tradeReadiness: verdict.decisionContext.trade_readiness,
            elapsedMs: Date.now() - startedAt,
        };
    }

    function chipClass(outcome: PairOutcome): string {
        if (outcome.status === 'pending') return styles.chipPending;
        if (outcome.status === 'adding') return styles.chipAdding;
        if (outcome.status === 'waiting') return styles.chipWaiting;
        if (outcome.status === 'evaluating') return styles.chipWaiting;
        if (outcome.reason === 'KEEP') return styles.chipKeep;
        if (outcome.reason === 'DUPLICATE' || outcome.reason === 'INVALID' || outcome.reason === 'UNAVAILABLE') {
            return styles.chipSkipped;
        }
        return styles.chipRemoved;
    }

    function statusText(outcome: PairOutcome): string {
        if (outcome.status === 'pending') return 'Queued';
        if (outcome.status === 'adding') return 'Adding…';
        if (outcome.status === 'waiting') return 'Awaiting decision…';
        if (outcome.status === 'evaluating') return 'Evaluating…';
        if (outcome.reason === 'KEEP') {
            return `Kept · ${outcome.guidance ?? ''} · ${outcome.tradeReadiness ?? ''}`;
        }
        return reasonLabel(outcome.reason);
    }

    function progressLabel(): string {
        if (phase !== 'running') return '';
        const total = outcomes.length;
        const done = outcomes.filter((o) => o.status === 'done').length;
        if (total === 0) return '';
        if (done >= total) return `Done · ${total}/${total}`;
        return `Processing ${Math.min(done + 1, total)} of ${total}`;
    }
</script>

{#if isOpen}
    <div class={brutalistStyles.confirmOverlay} role="presentation" onclick={close}>
        <div
            class={phase === 'input' ? `${brutalistStyles.confirmDialog} ${styles.scannerDialog}` : `${brutalistStyles.confirmDialog} ${styles.scannerDialogRunning}`}
            role="dialog"
            aria-modal="true"
            tabindex="-1"
            onclick={(e) => e.stopPropagation()}
            onkeydown={(e) => e.stopPropagation()}
        >
            {#if phase === 'input'}
                <div class={styles.inputBlock}>
                    <label class={styles.inputLabel} for="watchlist-input">Watchlist symbols</label>
                    <textarea
                        id="watchlist-input"
                        class={styles.inputField}
                        placeholder="BTC ETH SOL #AVAX, OP ARB"
                        maxlength="800"
                        bind:value={inputText}
                        disabled={!sessionReady}
                    ></textarea>
                    <div class={styles.inputHelp}>
                        Paste a tag-style list. Spaces, commas, and # prefixes are all accepted; duplicates
                        are ignored. Up to 10 characters per symbol.
                    </div>
                    <div class={styles.inputMeta}>
                        <span class={styles.inputMetaLabel}>{parsed.length} queued</span>
                        <span class="{styles.inputMetaCount} {parsed.length === 0 ? styles.zero : ''}">
                            {parsed.length} symbol{parsed.length === 1 ? '' : 's'}
                        </span>
                    </div>
                </div>

                {#if !sessionReady}
                    <div class={styles.guardBanner}>
                        Initialize a session (select an exchange) before running the watchlist scanner.
                    </div>
                {/if}

                <div class={styles.actionsFooter}>
                    <button class={styles.cancelBtn} onclick={cancel}>Cancel</button>
                    <button
                        class={styles.continueBtn}
                        onclick={startRun}
                        disabled={!sessionReady || parsed.length === 0}
                    >
                        Continue
                    </button>
                </div>
            {:else if phase === 'running'}
                <div class={styles.runningHeader}>
                    <h3 class={styles.runningTitle}>Watchlist scan</h3>
                    <span class={styles.runningProgress}>{progressLabel()}</span>
                </div>
                <div class={styles.pairList}>
                    {#each outcomes as outcome, idx (outcome.pairKey)}
                        <div class={styles.pairRow}>
                            {#if outcome.status === 'waiting' || outcome.status === 'adding'}
                                <span class="{styles.dot} {styles.dotReady}"></span>
                            {:else if outcome.status === 'done'}
                                <span class={styles.dot}></span>
                            {:else}
                                <span class={styles.dot}></span>
                            {/if}
                            <span class={styles.pairSymbol}>{outcome.base}</span>
                            <span class={styles.pairStatus}>{statusText(outcome)}</span>
                            <span class="{styles.pairChip} {chipClass(outcome)}">
                                {#if outcome.status === 'pending'}Queued
                                {:else if outcome.status === 'adding'}Add
                                {:else if outcome.status === 'waiting'}Wait
                                {:else if outcome.reason === 'KEEP'}Keep
                                {:else}Remove{/if}
                            </span>
                        </div>
                    {/each}
                </div>
                <div class={styles.actionsFooter}>
                    <button class={styles.cancelBtn} onclick={cancel}>Cancel</button>
                </div>
            {:else if phase === 'done' && summary}
                <div class={styles.summaryBlock}>
                    <div class={styles.summaryStats}>
                        <div class={styles.summaryStat}>
                            <span class={styles.summaryStatLabel}>Added</span>
                            <span class={styles.summaryStatValue}>{summary.added}</span>
                        </div>
                        <div class={styles.summaryStat}>
                            <span class={styles.summaryStatLabel}>Kept</span>
                            <span class="{styles.summaryStatValue} {styles.kept}">{summary.kept.length}</span>
                        </div>
                        <div class={styles.summaryStat}>
                            <span class={styles.summaryStatLabel}>Removed</span>
                            <span class="{styles.summaryStatValue} {styles.removed}">{summary.removed.length}</span>
                        </div>
                        {#if summary.skipped.length > 0}
                            <div class={styles.summaryStat}>
                                <span class={styles.summaryStatLabel}>Skipped</span>
                                <span class="{styles.summaryStatValue} {styles.skipped}">{summary.skipped.length}</span>
                            </div>
                        {/if}
                    </div>

                    <div class={styles.summaryGroups}>
                        {#if summary.kept.length > 0}
                            <div class={styles.summaryGroup}>
                                <span class={styles.summaryGroupTitle}>Kept</span>
                                <div class={styles.summaryGroupItems}>
                                    {#each summary.kept as o (o.pairKey)}
                                        <span class={styles.summaryGroupItem}>
                                            {o.base} · {o.guidance ?? ''}
                                        </span>
                                    {/each}
                                </div>
                            </div>
                        {/if}
                        {#if summary.removed.length > 0}
                            <div class={styles.summaryGroup}>
                                <span class={styles.summaryGroupTitle}>Removed</span>
                                <div class={styles.summaryGroupItems}>
                                    {#each summary.removed as o (o.pairKey)}
                                        <span class={styles.summaryGroupItem}>
                                            {o.base} · {reasonLabel(o.reason)}
                                        </span>
                                    {/each}
                                </div>
                            </div>
                        {/if}
                        {#if summary.skipped.length > 0}
                            <div class={styles.summaryGroup}>
                                <span class={styles.summaryGroupTitle}>Skipped</span>
                                <div class={styles.summaryGroupItems}>
                                    {#each summary.skipped as o (o.pairKey)}
                                        <span class={styles.summaryGroupItem}>
                                            {o.base} · {reasonLabel(o.reason)}
                                        </span>
                                    {/each}
                                </div>
                            </div>
                        {/if}
                        {#if summary.kept.length === 0 && summary.removed.length === 0 && summary.skipped.length === 0}
                            <span class={styles.summaryEmpty}>No pairs were processed.</span>
                        {/if}
                    </div>
                </div>

                <div class={styles.actionsFooter}>
                    <button class={styles.acceptBtn} onclick={close}>Accept</button>
                </div>
            {/if}
        </div>
    </div>
{/if}
