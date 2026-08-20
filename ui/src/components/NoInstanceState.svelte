<script lang="ts">
    // NoInstanceState — the shared "no active instance" empty state (v7.3).
    //
    // Mirrors the MME InstancePicker empty state: an SVG icon + a title +
    // engine-specific guidance. Used by TAE / PME / PAE whenever no
    // instance is active — data surfaces must NEVER show fallback data or
    // a loading message in this state. The Settings tab is exempt (it
    // renders instance-independent config).
    import SvgIcon from '../lib/SvgIcon.svelte';
    import styles from './NoInstanceState.module.css';

    interface Props {
        /** Engine key drives the guidance copy. */
        engine: 'trade_automation' | 'portfolio' | 'performance';
    }

    let { engine }: Props = $props();

    const COPY: Record<Props['engine'], { title: string; body: string }> = {
        trade_automation: {
            title: 'No active instance',
            body: 'Trade automation runs per instance. Launch one from the Instances panel (top-right) to see setups, orders, activity and trade history.',
        },
        portfolio: {
            title: 'No active instance',
            body: 'Portfolio management runs per instance. Launch one from the Instances panel (top-right) to see positions, exposure, capital, portfolio and safety.',
        },
        performance: {
            title: 'No active instance',
            body: 'Performance analytics evaluate recorded decisions per instance. Launch one from the Instances panel (top-right) to run backtests and review the edge.',
        },
    };
</script>

<div class={styles.state} role="status">
    <div class={styles.icon}><SvgIcon name="layoutDashboard" size={44} /></div>
    <h3 class={styles.title}>{COPY[engine].title}</h3>
    <p class={styles.body}>{COPY[engine].body}</p>
</div>
