export class SessionStore {
    sessionActive = $state(false);
    sessionCurrency = $state<string>('USDT');
    sessionExchange = $state<string>('Hyperliquid');
    sessionCapital = $state(10000);
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
                this.sessionCapital = data.capital || 10000;
                this.sessionInstanceCount = data.instance_count || 0;
            }
        } catch (_) { /* backend may not be ready yet */ } finally { this.sessionChecked = true; }
    }

    async initSession(currency: string, exchange: string): Promise<{ success: boolean; error?: string }> {
        this.sessionLoading = true; this.sessionError = null;
        try {
            const res = await fetch('/api/session/init', {
                method: 'POST', headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ currency, exchange }),
            });
            const data = await res.json();
            if (res.ok && data.success) {
                const wasActive = this.sessionActive;
                this.sessionActive = true; this.sessionCurrency = currency;
                this.sessionExchange = exchange;
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
                this.sessionExchange = 'Hyperliquid'; this.sessionInstanceCount = 0;
                this.sessionLoading = false; return true;
            }
        } catch (_) {}
        this.sessionLoading = false; return false;
    }
}
