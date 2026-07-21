<script lang="ts">
    // DerivativeRibbon — horizontal bar of 6 perp derivative badges with
    // per-badge tri-state feed status (CONNECTING → LIVE → STALE).
    //
    // Reads live from `tf.latestSnapshot.indicators` because every field
    // (OI, OI Δ, funding, OFI, spread, depth bias) is broadcast on the WS
    // envelope. The "stale" detector tracks each metric's last-update
    // timestamp and compares it against the broadcast cadence; metrics
    // that have never received a non-null value stay in "CONNECTING" and
    // make it clear the data feed hasn't ticked yet (HL derivatives
    // poller, WS order book stream, etc.) instead of looking like the
    // value is genuinely zero.
    import { useAppStore } from '../state.svelte';
    import styles from './DerivativeRibbon.module.css';
    import { iRaw, fmt, fmtPrice } from '../lib/telemetry';
    import type { IndicatorMap } from '../types';

    const app = useAppStore();
    let { slot }: { slot: 'micro' | 'fast' | 'slow' | 'macro' } = $props();

    const pair = $derived(app.instancesMap[app.activeTab] ?? null);
    const tf = $derived(
        slot === 'micro' ? pair?.microTerm :
        slot === 'fast'  ? pair?.fastTerm :
        slot === 'slow'  ? pair?.slowTerm :
                          pair?.macroTerm
    );

    const snap = $derived(tf?.latestSnapshot ?? null);
    const indicators = $derived<IndicatorMap>((snap?.indicators ?? {}) as IndicatorMap);

    const oiRaw = $derived<number | null>(iRaw(indicators, 'open_interest'));
    const oiDeltaRaw = $derived<number | null>(iRaw(indicators, 'oi_delta'));
    const fundingRaw = $derived<number | null>(iRaw(indicators, 'funding_rate'));
    const ofiRaw = $derived<number | null>(iRaw(indicators, 'order_flow_imbalance'));
    const spreadRaw = $derived<number | null>(iRaw(indicators, 'spread'));
    const depthRaw = $derived<number | null>(iRaw(indicators, 'depth_bias'));

    /// Tri-state feed status logic:
    /// - CONNECTING: no value ever received → expected during cold start.
    /// - LIVE:       value present and `now - lastUpdate < STALE_THRESHOLD_MS`.
    /// - STALE:      value present but broadcast cadence stalled.
    type FeedStatus = 'CONNECTING' | 'LIVE' | 'STALE';
    const STALE_THRESHOLD_MS = 30_000;

    function computeStatus(raw: number | null | undefined, lastUpdate: number | null): FeedStatus {
        if (raw == null) return 'CONNECTING';
        if (lastUpdate == null) return 'CONNECTING';
        if (Date.now() - lastUpdate > STALE_THRESHOLD_MS) return 'STALE';
        return 'LIVE';
    }

    /// Per-metric last-update tracker. Keyed by the WS envelope's
    /// `timestamp` field so a live ticker bump moves the cursor even if
    /// the metric value itself stays at the same number.
    let lastSeen = $state<Record<string, number | null>>({
        open_interest: null,
        oi_delta: null,
        funding_rate: null,
        order_flow_imbalance: null,
        spread: null,
        depth_bias: null,
    });

    $effect(() => {
        const ts = (snap?.timestamp ?? null) as number | null;
        if (ts == null || ts <= 0) return;
        // Snapshot delivered — bump every metric to "this is the last time
        // we heard about you", regardless of whether the value is null.
        lastSeen = {
            open_interest: ts,
            oi_delta: ts,
            funding_rate: ts,
            order_flow_imbalance: ts,
            spread: ts,
            depth_bias: ts,
        };
    });

    const oiStatus = $derived(computeStatus(oiRaw, lastSeen['open_interest']));
    const oiDeltaStatus = $derived(computeStatus(oiDeltaRaw, lastSeen['oi_delta']));
    const fundingStatus = $derived(computeStatus(fundingRaw, lastSeen['funding_rate']));
    const ofiStatus = $derived(computeStatus(ofiRaw, lastSeen['order_flow_imbalance']));
    const spreadStatus = $derived(computeStatus(spreadRaw, lastSeen['spread']));
    const depthStatus = $derived(computeStatus(depthRaw, lastSeen['depth_bias']));

    const oiFmt = $derived(() => {
        if (oiRaw == null) return '--';
        if (oiRaw >= 1_000_000_000) return `${(oiRaw / 1_000_000_000).toFixed(2)}B`;
        if (oiRaw >= 1_000_000) return `${(oiRaw / 1_000_000).toFixed(2)}M`;
        if (oiRaw >= 1_000) return `${(oiRaw / 1_000).toFixed(1)}K`;
        return oiRaw.toFixed(0);
    });

    const oiDeltaCls = $derived(
        oiDeltaRaw == null ? styles.neutral :
        oiDeltaRaw > 0 ? styles.bullish :
        oiDeltaRaw < 0 ? styles.bearish :
        styles.neutral
    );

    const fundingCls = $derived(
        fundingRaw == null ? styles.neutral :
        fundingRaw >= 0.03 ? styles.bearish :
        fundingRaw <= -0.03 ? styles.bullish :
        styles.neutral
    );

    const ofiCls = $derived(
        ofiRaw == null ? styles.neutral :
        ofiRaw > 0.1 ? styles.bullish :
        ofiRaw < -0.1 ? styles.bearish :
        styles.neutral
    );

    const spreadCls = $derived(
        spreadRaw == null ? styles.neutral :
        spreadRaw > 0.3 ? styles.neutral :
        styles.neutral
    );

    const depthCls = $derived(
        depthRaw == null ? styles.neutral :
        depthRaw > 0.15 ? styles.bullish :
        depthRaw < -0.15 ? styles.bearish :
        styles.neutral
    );

    const fundingSub = $derived(
        fundingRaw == null ? `${fundingStatus} · AWAITING POLLER` :
        fundingRaw >= 0.03 ? 'EXT+ LONG CROWDED' :
        fundingRaw <= -0.03 ? 'EXT- SHORT CROWDED' :
        Math.abs(fundingRaw) < 0.005 ? 'NEUTRAL' :
        fundingRaw > 0 ? 'LONG PAYING' : 'SHORT PAYING'
    );

    const oiDeltaSub = $derived(
        oiDeltaRaw == null ? `${oiDeltaStatus} · NO DELTA` :
        oiDeltaRaw > 0 ? 'OI RISING' :
        oiDeltaRaw < 0 ? 'OI FALLING' :
        'FLAT'
    );

    const ofiSub = $derived(
        ofiRaw == null ? `${ofiStatus} · AWAITING BOOK` :
        ofiRaw > 0.1 ? 'BUY PRESSURE' :
        ofiRaw < -0.1 ? 'SELL PRESSURE' :
        'BALANCED'
    );

    const depthSub = $derived(
        depthRaw == null ? `${depthStatus} · AWAITING BOOK` :
        depthRaw > 0.15 ? 'BID HEAVY' :
        depthRaw < -0.15 ? 'ASK HEAVY' :
        'BALANCED'
    );

    const spreadSub = $derived(
        spreadRaw == null ? `${spreadStatus} · NO TICK` :
        `${spreadRaw.toFixed(3)}%`
    );

    const oiBgCls = $derived(
        oiRaw == null ? styles.connecting :
        oiDeltaRaw != null && oiDeltaRaw > 0 ? styles.bullishBg :
        oiDeltaRaw != null && oiDeltaRaw < 0 ? styles.bearishBg :
        ''
    );

    const fundingBgCls = $derived(
        fundingRaw == null ? styles.connecting :
        Math.abs(fundingRaw) >= 0.03 ? styles.warningBg :
        ''
    );

    function statusClass(status: FeedStatus): string {
        if (status === 'LIVE') return styles.statusLive;
        if (status === 'STALE') return styles.statusStale;
        return styles.statusConnecting;
    }
    function statusText(status: FeedStatus): string {
        if (status === 'LIVE') return 'LIVE';
        if (status === 'STALE') return 'STALE';
        return 'CONNECTING';
    }
</script>

<div class={styles.ribbon} role="region" aria-label="Derivative Telemetry">
    <span class={styles.ribbonLabel}>DERIVATIVES</span>

    <div class="{styles.badge} {oiBgCls}">
        <div class={styles.badgeHeader}>
            <span class={styles.badgeName}>Open Interest</span>
            <span class="{styles.feedStatus} {statusClass(oiStatus)}">{statusText(oiStatus)}</span>
        </div>
        <span class={styles.badgeValue}>{oiFmt()}</span>
        <span class={styles.badgeSub}>{oiDeltaSub}</span>
    </div>

    <div class={styles.badge}>
        <div class={styles.badgeHeader}>
            <span class={styles.badgeName}>OI Δ</span>
            <span class="{styles.feedStatus} {statusClass(oiDeltaStatus)}">{statusText(oiDeltaStatus)}</span>
        </div>
        <span class="{styles.badgeValue} {oiDeltaCls}">
            {oiDeltaRaw == null ? '--' : (oiDeltaRaw >= 0 ? '+' : '') + fmt(oiDeltaRaw, 2)}
        </span>
        <span class={styles.badgeSub}>{oiDeltaSub}</span>
    </div>

    <div class="{styles.badge} {fundingBgCls}">
        <div class={styles.badgeHeader}>
            <span class={styles.badgeName}>Funding 8h</span>
            <span class="{styles.feedStatus} {statusClass(fundingStatus)}">{statusText(fundingStatus)}</span>
        </div>
        <span class="{styles.badgeValue} {fundingCls}">
            {fundingRaw == null ? '--' : `${(fundingRaw * 100).toFixed(4)}%`}
        </span>
        <span class={styles.badgeSub}>{fundingSub}</span>
    </div>

    <div class={styles.badge}>
        <div class={styles.badgeHeader}>
            <span class={styles.badgeName}>Order Flow</span>
            <span class="{styles.feedStatus} {statusClass(ofiStatus)}">{statusText(ofiStatus)}</span>
        </div>
        <span class="{styles.badgeValue} {ofiCls}">
            {ofiRaw == null ? '--' : fmt(ofiRaw, 2)}
        </span>
        <span class={styles.badgeSub}>{ofiSub}</span>
    </div>

    <div class={styles.badge}>
        <div class={styles.badgeHeader}>
            <span class={styles.badgeName}>Spread</span>
            <span class="{styles.feedStatus} {statusClass(spreadStatus)}">{statusText(spreadStatus)}</span>
        </div>
        <span class="{styles.badgeValue} {spreadCls}">
            {spreadRaw == null ? '--' : `${spreadRaw.toFixed(3)}%`}
        </span>
        <span class={styles.badgeSub}>{spreadSub}</span>
    </div>

    <div class={styles.badge}>
        <div class={styles.badgeHeader}>
            <span class={styles.badgeName}>Depth Bias</span>
            <span class="{styles.feedStatus} {statusClass(depthStatus)}">{statusText(depthStatus)}</span>
        </div>
        <span class="{styles.badgeValue} {depthCls}">
            {depthRaw == null ? '--' : fmt(depthRaw, 2)}
        </span>
        <span class={styles.badgeSub}>{depthSub}</span>
    </div>
</div>
