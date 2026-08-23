// lifecyclePresentation — the single vocabulary source for the instance
// lifecycle (v10.1).
//
// One state machine drives TAE activation everywhere:
//   ACTIVE    (RUNNING)  — the TAE opens new setups AND manages positions.
//   PAUSED    (PAUSED)   — close-only: open positions exit by their rules;
//                          no new setups. (TAE deactivated.)
//   FLATTENING (STOPPING) — transitional market close in progress.
//   TERMINATED (STOPPED) — flat, no trading; delete removes the instance.
//   MONITORING (observe) — ghost radar, never dispatches.
//
// Every surface (panel chips, TAE header switch, badges) imports from here
// so the language can never drift.

export type LifecycleToken = 'RUNNING' | 'PAUSED' | 'STOPPING' | 'STOPPED';

export interface LifecyclePresentation {
    label: string;
    color: string;
    description: string;
}

export const LIFECYCLE_PRESENTATION: Record<LifecycleToken, LifecyclePresentation> = {
    RUNNING: {
        label: 'ACTIVE',
        color: '#22c55e',
        description: 'TAE opens new setups and manages open positions.',
    },
    PAUSED: {
        label: 'PAUSED',
        color: '#f59e0b',
        description: 'Close-only — open positions exit by their rules; no new setups.',
    },
    STOPPING: {
        label: 'FLATTENING',
        color: '#f59e0b',
        description: 'Closing open positions at market.',
    },
    STOPPED: {
        label: 'TERMINATED',
        color: 'rgba(255,255,255,0.4)',
        description: 'Flat, no trading. Delete removes the instance.',
    },
};

export const MONITORING_PRESENTATION: LifecyclePresentation = {
    label: 'MONITORING',
    color: 'rgba(255,255,255,0.45)',
    description: 'Observe mode — ghost radar only, never dispatches.',
};

/** Resolve the display presentation for a backend lifecycle token. */
export function lifecyclePresentation(
    token: string | null | undefined,
    mode?: 'observe' | 'paper' | 'live' | string | null | undefined,
): LifecyclePresentation {
    if (mode === 'observe') return MONITORING_PRESENTATION;
    const key = (token ?? '').toUpperCase() as LifecycleToken;
    return LIFECYCLE_PRESENTATION[key] ?? MONITORING_PRESENTATION;
}

/** True when the activation toggle should be exposed (paper/live only). */
export function isActivatable(mode?: string | null | undefined): boolean {
    return mode === 'paper' || mode === 'live';
}

/** True when the instance currently takes new setups. */
export function isActive(token: string | null | undefined): boolean {
    return (token ?? '').toUpperCase() === 'RUNNING';
}
