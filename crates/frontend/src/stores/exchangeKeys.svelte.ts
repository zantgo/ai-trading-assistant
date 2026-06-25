import type { ExchangeAccount } from '../types';

export class ExchangeKeyStore {
    exchangeAccounts = $state<ExchangeAccount[]>([]);
    exchangeActiveCount = $state(0);
    exchangeMaxAccounts = $state(3);
    exchangeFormDraft = $state<{
        exchange: string; account_name: string; api_key: string; api_secret: string;
        passphrase: string; referred_uid: string; is_active: boolean;
    }>({
        exchange: 'Bitget', account_name: '', api_key: '', api_secret: '',
        passphrase: '', referred_uid: '', is_active: true,
    });

    async fetchExchangeKeys() {
        try { const res = await fetch('/api/exchange-keys'); if (res.ok) { this.exchangeAccounts = await res.json(); this.exchangeActiveCount = this.exchangeAccounts.filter(a => a.is_active).length; } } catch (_) {}
    }

    async addExchangeKey() {
        try { const res = await fetch('/api/exchange-keys', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(this.exchangeFormDraft) }); if (res.ok) { this.exchangeFormDraft = { exchange: 'Bitget', account_name: '', api_key: '', api_secret: '', passphrase: '', referred_uid: '', is_active: true }; await this.fetchExchangeKeys(); } } catch (_) {}
    }

    async deleteExchangeKey(id: number) {
        try { await fetch(`/api/exchange-keys/${id}`, { method: 'DELETE' }); await this.fetchExchangeKeys(); } catch (_) {}
    }
}
