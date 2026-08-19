export class SessionStore {
    sessionActive = $state(false);
    sessionCurrency = $state<string>('USDT');
    sessionExchange = $state<string>('Hyperliquid');
    sessionCapital = $state(1000);
    sessionMode = $state<'paper' | 'live'>('paper');
    sessionInstanceCount = $state(0);
    sessionLoading = $state(false);
    sessionChecked = $state(false);
    sessionError = $state<string | null>(null);

    onSessionActivated: (() => void) | null = null;

    async fetchSessionStatus() {
        try {
            const res = await fetch('/api/session/status');
            if (res.ok) {
                const data = await res.json();
                const wasActive = this.sessionActive;
                this.sessionActive = data.active;
                if (this.sessionActive && !wasActive && this.onSessionActivated) this.onSessionActivated();
                this.sessionCurrency = data.currency || 'USDT';
                this.sessionExchange = data.exchange || 'Hyperliquid';
                if (data.capital) this.sessionCapital = data.capital;
                if (data.mode) this.sessionMode = data.mode;
                this.sessionInstanceCount = data.instance_count || 0;
            }
        } catch (_) { /* backend may not be ready yet */ } finally { this.sessionChecked = true; }
    }

    async initSession(
        currency: string,
        exchange: string,
        mode: 'paper' | 'live' = 'paper',
        capital?: number,
    ): Promise<{ success: boolean; error?: string }> {
        this.sessionLoading = true; this.sessionError = null;
        try {
            const body: Record<string, unknown> = { currency, exchange, mode };
            if (capital != null && capital > 0) body.initial_capital_usd = capital;
            const res = await fetch('/api/session/init', {
                method: 'POST', headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(body),
            });
            const data = await res.json();
            if (res.ok && data.success) {
                const wasActive = this.sessionActive;
                this.sessionActive = true; this.sessionCurrency = currency;
                this.sessionExchange = exchange; this.sessionMode = mode;
                if (capital != null && capital > 0) this.sessionCapital = capital;
                if (!wasActive && this.onSessionActivated) this.onSessionActivated();
                this.sessionLoading = false; return { success: true };
            }
            this.sessionError = data.error || 'Session initialization failed';
        } catch (e: any) { this.sessionError = e.message || 'Network error'; }
        this.sessionLoading = false;
        return { success: false, error: this.sessionError || undefined };
    }

    async quitSession(): Promise<boolean> {
        this.sessionLoading = true;
        try {
            const res = await fetch('/api/session/quit', { method: 'POST' });
            const data = await res.json();
            if (res.ok && data.success) {
                this.sessionActive = false; this.sessionCurrency = 'USDT';
                this.sessionExchange = 'Hyperliquid';
                this.sessionMode = 'paper'; this.sessionInstanceCount = 0;
                this.sessionLoading = false; return true;
            }
        } catch (_) {}
        this.sessionLoading = false; return false;
    }
}
