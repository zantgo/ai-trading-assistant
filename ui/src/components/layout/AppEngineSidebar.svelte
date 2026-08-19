<script lang="ts">
    import { useAppStore } from '../../state.svelte';
    import SvgIcon from '../../lib/SvgIcon.svelte';
    import { buildEngineHash } from '../../lib/router.svelte';
    import styles from '../../styles/brutalist-grid.module.css';

    interface Props {
        isOpen: boolean;
        currentEngine: string;
        onclose: () => void;
        onnavigate: (engine: string) => void;
        onquit: () => void;
    }

    let { isOpen, currentEngine, onclose, onnavigate, onquit }: Props = $props();

    type EngineKey = 'profile' | 'data_infra' | 'market_monitor' | 'trade_automation' | 'portfolio' | 'performance' | 'exchange_settings';

    const ENGINES_SIDEBAR: { key: EngineKey; label: string; divider?: boolean }[] = [
        { key: 'data_infra',        label: 'Data Infrastructure' },
        { key: 'market_monitor',    label: 'Market Monitor' },
        { key: 'trade_automation',  label: 'Trade Automation' },
        { key: 'portfolio',         label: 'Portfolio Management' },
        { key: 'performance',       label: 'Performance Analytics' },
        { key: 'profile', label: 'Settings', divider: true },
    ];

    function sidebarItemClass(key: EngineKey): string {
        const base = styles.sidebarItem;
        return currentEngine === key ? `${base} ${styles.sidebarItemActive}` : base;
    }

    function sidebarSvg(key: EngineKey): string {
        return '';
    }

    function sidebarIconName(key: EngineKey): string {
        const map: Record<EngineKey, string> = {
            profile: 'settings',
            data_infra: 'database',
            market_monitor: 'trend',
            trade_automation: 'cycle',
            portfolio: 'dollar',
            performance: 'search',
            exchange_settings: 'key',
        };
        return map[key] || 'home';
    }

    function handleNavClick(e: MouseEvent) {
        if (e.button !== 0 || e.ctrlKey || e.metaKey || e.shiftKey) return;
        e.preventDefault();
    }

</script>

{#if isOpen}
    <div class={styles.sidebarOverlay} role="presentation" onclick={onclose}></div>
    <div class={styles.sidebarPanel}>
        <div class={styles.sidebarBrand}>TRADING PLATFORM</div>
        <div class={styles.sidebarNav}>
            {#each ENGINES_SIDEBAR as engine (engine.key)}
                {#if engine.divider}
                    <div class={styles.sidebarDivider}></div>
                {/if}
                <a href={buildEngineHash(engine.key)} class={sidebarItemClass(engine.key)} onclick={(e) => { handleNavClick(e); onnavigate(engine.key); }}>
                    <span class={styles.navIcon}><SvgIcon name={sidebarIconName(engine.key)} size={15} /></span>{engine.label}
                </a>
            {/each}
        </div>
        <div class={styles.sidebarFooter}>
            <button class={styles.sidebarQuitBtn} onclick={() => { onclose(); onquit(); }}>
                <span class={styles.navIcon}><SvgIcon name="logout" size="sm" /></span>
                Quit Session
            </button>
        </div>
    </div>
{/if}
