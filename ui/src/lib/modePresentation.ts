// modePresentation — single source of truth for execution-mode labels,
// chips and banner copy shared by TAE / PME / PAE dashboards. The mode is
// fixed at launch (Launch Setup wizard) and never toggled at runtime.

export type ExecutionMode = 'observe' | 'paper' | 'live';

export type ModeEngine = 'trade_automation' | 'portfolio' | 'performance';

export const MODE_LABEL: Record<ExecutionMode, string> = {
    observe: 'OBSERVE',
    paper: 'PAPER',
    live: 'LIVE',
};

export function isExecutionMode(v: unknown): v is ExecutionMode {
    return v === 'observe' || v === 'paper' || v === 'live';
}

export function modeBannerCopy(engine: ModeEngine, mode: ExecutionMode | undefined): string {
    if (mode === 'observe') {
        if (engine === 'trade_automation') {
            return 'OBSERVE — monitoring only. No orders are dispatched. The radar shows what the executor WOULD do.';
        }
        if (engine === 'portfolio') {
            return 'No capital engaged — readiness view. The protection system is shown but unarmed.';
        }
        return 'Edge validation — no capital deployed. Statistical significance is the focus.';
    }
    if (mode === 'paper') {
        if (engine === 'trade_automation') {
            return 'PAPER — simulated orders against paper capital. Nothing touches a real venue.';
        }
        if (engine === 'portfolio') {
            return 'PAPER capital — all portfolio figures are simulated.';
        }
        return 'PAPER record — analytics blend backtest and forward-tested paper results.';
    }
    if (mode === 'live') {
        if (engine === 'trade_automation') {
            return 'LIVE — real funds on the exchange. Verify venue states below before acting.';
        }
        if (engine === 'portfolio') {
            return 'LIVE funds — portfolio figures reflect the real account ledger.';
        }
        return 'LIVE performance — real-money results included in the analytics.';
    }
    return 'Execution mode unavailable.';
}
