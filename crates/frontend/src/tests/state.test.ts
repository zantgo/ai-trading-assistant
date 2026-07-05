// @vitest-environment jsdom
import { describe, it, expect, beforeEach } from 'vitest';
import { useAppStore } from '../state.svelte';

describe('TEST-UI: Global State Runes', () => {
    let app: ReturnType<typeof useAppStore>;

    beforeEach(() => {
        app = useAppStore();
        app.analysisPhase = 'idle';
        app.currentPosition = 'None';
        app.entryPriceVal = '';
        app.isAssistantModalOpen = false;
        app.chatHistory = [];
    });

    it('should initialize with default states', () => {
        expect(app.analysisPhase).toBe('idle');
        expect(app.currentPosition).toBe('None');
        expect(app.isAssistantModalOpen).toBe(false);
    });

    it('should handle position changes and validate fields', () => {
        app.currentPosition = 'Long';
        expect(app.currentPosition).toBe('Long');

        app.entryPriceVal = '3120.50';
        expect(app.entryPriceVal).toBe('3120.50');
    });

    it('should transition analysis phases progressively', () => {
        expect(app.analysisPhase).toBe('idle');

        app.analysisPhase = 'phase1';
        expect(app.analysisPhase).toBe('phase1');

        app.analysisPhase = 'phase2';
        expect(app.analysisPhase).toBe('phase2');

        app.analysisPhase = 'complete';
        expect(app.analysisPhase).toBe('complete');
    });

    it('should build chat history context correctly upon modal open', () => {
        app.chatHistory.push({ role: 'assistant', content: 'Greeting message' });
        expect(app.chatHistory.length).toBe(1);
        expect(app.chatHistory[0].role).toBe('assistant');
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

    it('should toggle apiKeyConfigured flag', () => {
        expect(app.apiKeyConfigured).toBe(true);
        app.apiKeyConfigured = false;
        expect(app.apiKeyConfigured).toBe(false);
        app.apiKeyConfigured = true;
        expect(app.apiKeyConfigured).toBe(true);
    });

    it('should restore per-mode Level 3 view when switching Level 2 modes', () => {
        app.initInstance('BTC');
        app.activeTab = 'BTC-USDT';

        app.switchMode('user');
        expect(app.currentLevel2Mode).toBe('user');
        expect(app.currentView).toBe('terminal');

        app.currentView = 'costs';
        expect(app.currentView).toBe('costs');

        app.switchMode('ai');
        expect(app.currentView).toBe('assistant');
        app.currentView = 'ledger';

        app.switchMode('user');
        expect(app.currentView).toBe('costs');

        app.switchMode('ai');
        expect(app.currentView).toBe('ledger');
    });

    it('should map Level 2 paradigms to backend operational modes', () => {
        app.initInstance('BTC');
        app.activeTab = 'BTC-USDT';

        app.switchMode('general');
        expect(app.pendingOperationalMode).toBe(null);
        app.switchMode('user');
        expect(app.pendingOperationalMode).toBe('ManualOnly');
        app.switchMode('rule');
        expect(app.pendingOperationalMode).toBe('DeterministicHeuristics');
        app.switchMode('ai');
        expect(app.pendingOperationalMode).toBe('HybridAiCopilot');
    });
});
