<script lang="ts">
    import { onDestroy } from 'svelte';
    import { useAppStore } from '../state.svelte';
    import OrderTicket from './OrderTicket.svelte';
    import PositionPerformanceChart from './PositionPerformanceChart.svelte';
    import BottomTable from './BottomTable.svelte';
    import styles from './PaperTradingPanel.module.css';

    const app = useAppStore();

    $effect(() => {
        app.fetchPaperStatus();
        app.fetchPaperHistory();
        app.fetchOpenOrders();
        app.fetchSlotStates();
        app.fetchEquityHistory();
    });

    const pollInterval = setInterval(() => {
        app.fetchPaperStatus();
        app.fetchOpenOrders();
        app.fetchSlotStates();
    }, 5000);

    onDestroy(() => clearInterval(pollInterval));
</script>

<div class={styles.positionsWorkspace}>
    <!-- Top Row: Order Ticket (left) + Performance Chart (right) -->
    <div class={styles.topControlsRow}>
        <div class={styles.orderTicketPanel}>
            <OrderTicket />
        </div>
        <div class={styles.performanceChartPanel}>
            <PositionPerformanceChart />
        </div>
    </div>

    <!-- Bottom Row: Unified Positions Table -->
    <div class={styles.bottomTableRow}>
        <BottomTable />
    </div>
</div>
