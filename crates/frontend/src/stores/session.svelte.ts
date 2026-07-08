export class SessionStore {
    sessionActive = $state(false);
    sessionMode = $state<string>('paper');
    sessionCurrency = $state<string>('USDT');
    sessionExchange = $state<string>('Hyperliquid');
    sessionCapital = $state(0);
    sessionInstanceCount = $state(0);
    sessionMaxInstances = $state(100);
    sessionUserName = $state<string>('');
    sessionWalletAddress = $state<string>('');
    sessionLoading = $state(false);
    sessionChecked = $state(false);
    sessionError = $state<string | null>(null);

    onSessionActivated: (() => void) | null = null;

    async fetchSessionStatus() {
        try {
            const res = await fetch('/api/session/status');
            if (res.ok) {
                const data = await res.json();
                this.sessionActive = data.active;
                if (this.sessionActive && this.onSessionActivated) this.onSessionActivated();
                this.sessionMode = data.mode || 'paper';
                this.sessionCurrency = data.currency || 'USDT';
                this.sessionExchange = data.exchange || 'Hyperliquid';
                this.sessionCapital = data.capital || 0;
                this.sessionInstanceCount = data.instance_count || 0;
                this.sessionMaxInstances = data.max_instances || 100;
                this.sessionUserName = data.user_name || '';
                this.sessionWalletAddress = data.wallet_address || '';
            }
        } catch (_) { /* backend may not be ready yet */ } finally { this.sessionChecked = true; }
    }

    async initSession(mode: string, currency: string, exchange: string, capital: number, userName: string): Promise<{ success: boolean; error?: string }> {
        this.sessionLoading = true; this.sessionError = null;
        try {
            const res = await fetch('/api/session/init', {
                method: 'POST', headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ mode, currency, exchange, capital, user_name: userName || undefined }),
            });
            const data = await res.json();
            if (res.ok && data.success) {
                this.sessionActive = true; this.sessionMode = mode; this.sessionCurrency = currency;
                this.sessionExchange = exchange; this.sessionCapital = capital;
                this.sessionUserName = userName;
                if (this.onSessionActivated) this.onSessionActivated();
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
                this.sessionActive = false; this.sessionMode = 'paper'; this.sessionCurrency = 'USDT';
                this.sessionExchange = 'Hyperliquid'; this.sessionCapital = 0; this.sessionInstanceCount = 0;
                this.sessionUserName = '';
                this.sessionLoading = false; return true;
            }
        } catch (_) {}
        this.sessionLoading = false; return false;
    }

    /** Save profile settings (name, wallet) without re-initializing session */
    async saveProfile(userName: string, walletAddress: string): Promise<boolean> {
        try {
            const res = await fetch('/api/settings/profile', {
                method: 'POST', headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ user_name: userName || undefined, wallet_address: walletAddress || undefined }),
            });
            if (res.ok) {
                this.sessionUserName = userName;
                this.sessionWalletAddress = walletAddress;
                return true;
            }
        } catch (_) {}
        return false;
    }

    /** Fetch profile fields from server */
    async fetchProfile(): Promise<{ userName: string; walletAddress: string } | null> {
        try {
            const res = await fetch('/api/settings/profile');
            if (res.ok) {
                const data = await res.json();
                this.sessionUserName = data.user_name || '';
                this.sessionWalletAddress = data.wallet_address || '';
                return { userName: this.sessionUserName, walletAddress: this.sessionWalletAddress };
            }
        } catch (_) {}
        return null;
    }
}
