import { describe, it, expect } from 'vitest';
import { formatRelativeTime } from './relTime';

describe('formatRelativeTime', () => {
    const now = 1_700_000_000_000;

    it('returns "—" for null', () => {
        expect(formatRelativeTime(null, now)).toEqual({ label: '—', seconds: NaN });
    });
    it('returns "—" for undefined', () => {
        expect(formatRelativeTime(undefined, now)).toEqual({ label: '—', seconds: NaN });
    });
    it('returns "—" for NaN', () => {
        expect(formatRelativeTime(NaN, now)).toEqual({ label: '—', seconds: NaN });
    });
    it('returns "—" for ms=0', () => {
        expect(formatRelativeTime(0, now)).toEqual({ label: '—', seconds: NaN });
    });
    it('returns "now" for < 5s', () => {
        const ms = now - 2000;
        expect(formatRelativeTime(ms, now)).toEqual({ label: 'now', seconds: 2 });
    });
    it('returns "12s ago" for 12s', () => {
        const ms = now - 12_000;
        expect(formatRelativeTime(ms, now)).toEqual({ label: '12s ago', seconds: 12 });
    });
    it('returns "3m ago" for 3 min', () => {
        const ms = now - 3 * 60_000;
        expect(formatRelativeTime(ms, now)).toEqual({ label: '3m ago', seconds: 180 });
    });
    it('returns "5h ago" for 5 hr', () => {
        const ms = now - 5 * 3_600_000;
        expect(formatRelativeTime(ms, now)).toEqual({ label: '5h ago', seconds: 18_000 });
    });
    it('returns "—" for > 24h', () => {
        const ms = now - 25 * 3_600_000;
        expect(formatRelativeTime(ms, now).label).toBe('—');
    });
    it('clamps future timestamps to 0s', () => {
        const ms = now + 10_000;
        expect(formatRelativeTime(ms, now)).toEqual({ label: 'now', seconds: 0 });
    });
});
