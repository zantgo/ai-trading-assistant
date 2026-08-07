<script lang="ts">
    import { useAppStore } from '../state.svelte';
    import SvgIcon from '../lib/SvgIcon.svelte';
    import styles from './InstancePicker.module.css';

    const app = useAppStore();

    interface InstanceRow {
        id: string;
        pair: string;
        symbol: string;
        status: string;
    }

    let instances = $state<InstanceRow[]>([]);
    let loading = $state(true);

    async function fetchInstances() {
        await Promise.resolve();
        loading = true;
        try {
            const res = await fetch('/api/instances');
            if (res.ok) {
                const data = await res.json();
                instances = data.instances || [];
            }
        } catch (_) {}
        finally { loading = false; }
    }

    $effect(() => {
        fetchInstances();
    });

    function pairDisplay(pairKey: string): string {
        return pairKey.replace('-', '/');
    }

    function priceFor(pairKey: string): string {
        const inst = app.instancesMap[pairKey];
        if (!inst) return '--';
        const tfs = [inst.microTerm, inst.fastTerm, inst.slowTerm, inst.macroTerm];
        for (const tf of tfs) {
            const p = tf?.priceText;
            if (p && p !== '0' && p !== 'NaN' && parseFloat(p) > 0) return p;
        }
        return inst.microTerm?.priceText || '--';
    }

    function changeStr(pairKey: string): string {
        const inst = app.instancesMap[pairKey];
        if (!inst) return '';
        const tfs = [inst.microTerm, inst.fastTerm, inst.slowTerm, inst.macroTerm];
        for (const tf of tfs) {
            const snap = tf?.latestSnapshot;
            if (!snap) continue;
            const mid = parseFloat(String((snap as Record<string, unknown>).mid_price ?? ''));
            const prev = parseFloat(String((snap as Record<string, unknown>).prev_day_px ?? ''));
            if (!isFinite(mid) || !isFinite(prev) || prev === 0) continue;
            const age = (Date.now() / 1000) - ((snap as Record<string, unknown>).timestamp as number);
            if (age < 60) {
                const v = ((mid - prev) / prev) * 100;
                return (v > 0 ? '+' : '') + v.toFixed(2) + '%';
            }
        }
        return '';
    }

    function changeCls(v: string): string {
        if (v.startsWith('+')) return styles.changeUp;
        if (v.startsWith('-')) return styles.changeDown;
        return styles.changeFlat;
    }

    function statusClass(status: string): string {
        switch (status) {
            case 'running': return styles.statusRunning;
            case 'paused': return styles.statusPaused;
            case 'stopped': return styles.statusStopped;
            default: return styles.statusStopped;
        }
    }
</script>

<div class={styles.picker}>
    <div class={styles.header}>
        <h2 class={styles.title}>Instances</h2>
        <p class={styles.subtitle}>Select a workspace to view charts and metrics</p>
    </div>

    {#if loading}
        <div class={styles.loading}>Loading instances…</div>
    {:else if instances.length === 0}
        <div class={styles.empty}>
            <div class={styles.emptyIcon}><SvgIcon name="layoutDashboard" size={48} /></div>
            <p class={styles.emptyMsg}>No active instances. Open the Instances panel (top-right) to create one.</p>
        </div>
    {:else}
        <div class={styles.list}>
            {#each instances as inst (inst.id)}
                {@const pk = inst.pair}
                {@const chg = changeStr(pk)}
                <button class={styles.row} onclick={() => app.enterInstance(pk)}>
                    <span class="{styles.statusDot} {statusClass(inst.status)}"></span>
                    <div class={styles.rowInfo}>
                        <span class={styles.symbol}>{pairDisplay(pk)}</span>
                        <span class={styles.price}>{priceFor(pk)}</span>
                    </div>
                    {#if chg}
                        <span class="{styles.change} {changeCls(chg)}">{chg}</span>
                    {/if}
                </button>
            {/each}
        </div>
    {/if}
</div>
