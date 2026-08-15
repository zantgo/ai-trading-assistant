// @vitest-environment jsdom
//
// SnapshotSchedulerButton — bottom-left CTA in `GeneralDashboard`.
// Renders a button that opens the SnapshotSchedulerModal. The button
// itself owns the polling loop (3s) so the status pill (`ON · 12s ago` /
// `ON · 60s` / `OFF` / red `ERROR`) stays fresh even when the modal is
// closed.

import { cleanup, render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi, afterEach as ae } from 'vitest';
import SnapshotSchedulerButton from './SnapshotSchedulerButton.svelte';
import { useAppStore } from '../state.svelte';
import type { SnapshotExportStatus } from '../types';

function makeStatus(overrides: Partial<SnapshotExportStatus> = {}): SnapshotExportStatus {
    return {
        enabled: false,
        output_path: './snapshots',
        interval_secs: 60,
        max_snapshots_retained: 1000,
        tabs: ['metrics', 'mtf', 'alignment', 'opportunity', 'risk', 'analysis', 'advisory', 'decision', 'recommendation'],
        last_snapshot_at: null,
        total_snapshots_written: 0,
        last_error: null,
        last_instance_count: 0,
        ...overrides,
    };
}

beforeEach(() => {
    const app = useAppStore();
    app.snapshotExportStatus = null;
    app.lastSnapshotExportFetchMs = null;
    app.lastSnapshotExportErrorMs = null;
});

afterEach(() => {
    cleanup();
});

describe('SnapshotSchedulerButton', () => {
    it('renders the CTA label and OFF status when no status loaded', () => {
        render(SnapshotSchedulerButton);
        expect(screen.getByText('SCHEDULE SNAPSHOTS')).toBeTruthy();
        // Initial pill text is "OFF …" before any fetch returns.
        expect(screen.getByText(/OFF/)).toBeTruthy();
    });

    it('shows DISABLED pill when status.enabled=false', () => {
        const app = useAppStore();
        app.snapshotExportStatus = makeStatus({ enabled: false });
        render(SnapshotSchedulerButton);
        expect(screen.getByText('OFF')).toBeTruthy();
    });

    it('shows ON pill when enabled with no snapshot yet', () => {
        const app = useAppStore();
        app.snapshotExportStatus = makeStatus({ enabled: true, interval_secs: 60 });
        render(SnapshotSchedulerButton);
        // Pill text "ON · 60s" appears.
        expect(screen.getByText(/ON · 60s/)).toBeTruthy();
    });

    it('shows ERROR pill when last_error is set', () => {
        const app = useAppStore();
        app.snapshotExportStatus = makeStatus({
            enabled: true,
            last_error: 'write /foo: permission denied',
        });
        render(SnapshotSchedulerButton);
        expect(screen.getByText('ERROR')).toBeTruthy();
    });

    it('clicking opens the modal', async () => {
        const app = useAppStore();
        app.snapshotExportStatus = makeStatus({ enabled: true });
        const { container } = render(SnapshotSchedulerButton);
        const button = container.querySelector('button');
        expect(button).toBeTruthy();
        await button!.click();
        expect(screen.getByText('SCHEDULE SNAPSHOTS', { selector: 'h3' })).toBeTruthy();
        // Modal exposes the four live status fields.
        expect(screen.getByText('Total written')).toBeTruthy();
        expect(screen.getByText('Last snapshot')).toBeTruthy();
    });
});
