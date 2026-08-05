<script lang="ts">
    // UtcClockBadge — perps-exchange-style UTC clock.
    //
    // Renders `YYYY-MM-DD HH:MM:SS UTC` next to the MARKET OVERVIEW
    // header. Updates every second via $effect so the operator always
    // sees a fresh stamp. Uses the browser's `Date` (defaulting to UTC
    // via `Intl.DateTimeFormat`) — the NTP-monitored server clock is
    // sub-second accurate for pipeline use, but the visible clock drift
    // is negligible for a UI badge.
    import styles from './UtcClockBadge.module.css';

    let now = $state(new Date());

    $effect(() => {
        const id = setInterval(() => {
            now = new Date();
        }, 1000);
        return () => clearInterval(id);
    });

    const datePart = $derived(
        now.toLocaleDateString('en-CA', {
            timeZone: 'UTC',
            year: 'numeric',
            month: '2-digit',
            day: '2-digit',
        })
    );
    const timePart = $derived(
        now.toLocaleTimeString('en-GB', {
            timeZone: 'UTC',
            hour: '2-digit',
            minute: '2-digit',
            second: '2-digit',
            hour12: false,
        })
    );
</script>

<div class={styles.badge} title="Current exchange time (UTC)">
    <span class={styles.dot}></span>
    <span class={styles.date}>{datePart}</span>
    <span class={styles.time}>{timePart}</span>
    <span class={styles.zone}>UTC</span>
</div>
