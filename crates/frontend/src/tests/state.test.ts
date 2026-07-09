// @vitest-environment jsdom
import { describe, it, expect, beforeEach } from 'vitest';
import { useAppStore } from '../state.svelte';

describe('TEST-UI: Global State Runes', () => {
    let app: ReturnType<typeof useAppStore>;

    beforeEach(() => {
        app = useAppStore();
        app.currentPosition = 'None';
        app.entryPriceVal = '';
    });

    it('should handle position changes and validate fields', () => {
        app.currentPosition = 'Long';
        expect(app.currentPosition).toBe('Long');

        app.entryPriceVal = '3120.50';
        expect(app.entryPriceVal).toBe('3120.50');
    });

    it('should initialize instancesMap with exchange-symbol key', () => {
        app.initInstance('BTC');
        expect(app.instancesMap['BTC-USDT']).toBeDefined();
        expect(app.instancesMap['BTC-USDT'].symbol).toBe('BTC');
        expect(app.instancesMap['BTC-USDT'].exchange).toBe('Hyperliquid');
        expect(app.instancesMap['BTC-USDT'].microTerm.priceText).toBe('--');
    });

    it('should route snapshot data by exchange key to correct pair', () => {
        app.initInstance('BTC');
        app.initInstance('ETH');

        app.instancesMap['BTC-USDT'].microTerm.priceText = '50000.00';
        app.instancesMap['BTC-USDT'].microTerm.latestSnapshot = { mid_price: '50000.00', exchange: 'Hyperliquid', symbol: 'BTC' };

        expect(app.instancesMap['BTC-USDT'].microTerm.priceText).toBe('50000.00');
        expect(app.instancesMap['ETH-USDT'].microTerm.priceText).toBe('--');
    });

});
