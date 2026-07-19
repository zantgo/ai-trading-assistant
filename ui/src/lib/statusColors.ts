export interface ColorThresholds {
    excellent: number;
    good: number;
    moderate: number;
}

const DEFAULT_THRESHOLDS: ColorThresholds = {
    excellent: 90,
    good: 75,
    moderate: 50,
};

export interface StatusColors {
    /** Healthy — #22c55e */
    excellent: string;
    /** Acceptable — #84cc16 */
    good: string;
    /** Marginal — #f59e0b */
    moderate: string;
    /** Degraded — #ef4444 */
    poor: string;
    /** Inactive / no data — #666 */
    none: string;
}

export const COLORS: StatusColors = {
    excellent: '#22c55e',
    good: '#84cc16',
    moderate: '#f59e0b',
    poor: '#ef4444',
    none: '#666',
};

export function thresholdValue(
    value: number,
    thresholds: ColorThresholds = DEFAULT_THRESHOLDS,
): string {
    if (value >= thresholds.excellent) return COLORS.excellent;
    if (value >= thresholds.good) return COLORS.good;
    if (value >= thresholds.moderate) return COLORS.moderate;
    return COLORS.poor;
}

export function connectionColor(
    state: 'Connected' | 'Connecting' | 'Reconnecting' | 'Disconnected' | 'Disabled',
): string {
    switch (state) {
        case 'Connected': return COLORS.excellent;
        case 'Connecting':
        case 'Reconnecting': return COLORS.moderate;
        case 'Disabled': return '#6b7280';
        default: return COLORS.poor;
    }
}

export function heartbeatColor(secondsAgo: number): string {
    if (secondsAgo <= 0) return COLORS.none;
    if (secondsAgo < 30) return COLORS.excellent;
    if (secondsAgo < 60) return COLORS.moderate;
    return COLORS.poor;
}
