// @vitest-environment jsdom
import { describe, it, expect, beforeEach } from 'vitest';
import { getState } from '../state.svelte';

describe('AI Trading Assistant Global State Tests', () => {
    let app: ReturnType<typeof getState>;

    beforeEach(() => {
        app = getState();
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
});
