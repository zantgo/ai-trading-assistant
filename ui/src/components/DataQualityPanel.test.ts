// @vitest-environment jsdom
import { cleanup, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { PipelineReliabilityMetrics } from '../types';
import DataQualityPanel from './DataQualityPanel.svelte';

const report: PipelineReliabilityMetrics = {
    coverage: 0.985,
    gap_count: 3,
    outliers_rejected: 7,
    outliers_bypassed: 1,
    out_of_order_dropped: 0,
    total_candles_processed: 15240,
    reconstructed_candles: 12,
};

function mockResponse(data: PipelineReliabilityMetrics): Response {
    return {
        ok: true,
        status: 200,
        json: async () => data,
    } as Response;
}

afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
});

describe('DataQualityPanel', () => {
    it('renders_loading_state_initially', () => {
        vi.stubGlobal('fetch', vi.fn(() => new Promise<Response>(() => {})));
        render(DataQualityPanel);
        expect(screen.getByText('Loading…')).toBeTruthy();
    });

    it('renders_metrics_after_fetch', async () => {
        vi.stubGlobal('fetch', vi.fn().mockResolvedValue(mockResponse(report)));
        render(DataQualityPanel);
        expect(await screen.findByText('98.50%')).toBeTruthy();
        expect(screen.getByText('15,240')).toBeTruthy();
        expect(screen.getByText('3')).toBeTruthy();
        expect(screen.getByText('7')).toBeTruthy();
        expect(screen.getByText('0')).toBeTruthy();
        expect(screen.getByText('12')).toBeTruthy();
    });

    it('renders_poor_coverage_color', async () => {
        const poor: PipelineReliabilityMetrics = { ...report, coverage: 0.75 };
        vi.stubGlobal('fetch', vi.fn().mockResolvedValue(mockResponse(poor)));
        render(DataQualityPanel);
        expect(await screen.findByText('75.00%')).toBeTruthy();
    });

    it('renders_error_state_on_fetch_failure', async () => {
        vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new Error('network down')));
        render(DataQualityPanel);
        expect(await screen.findByText('Error: network down')).toBeTruthy();
    });

    it('renders_out_of_order_warning', async () => {
        const ooo: PipelineReliabilityMetrics = { ...report, out_of_order_dropped: 15 };
        vi.stubGlobal('fetch', vi.fn().mockResolvedValue(mockResponse(ooo)));
        render(DataQualityPanel);
        expect(await screen.findByText('15')).toBeTruthy();
    });
});
