
type EngineKey = 'profile' | 'data_infra' | 'market_monitor' | 'trade_automation' | 'portfolio' | 'performance' | 'backtesting' | 'exchange_settings';

interface RouteParams {
    engine: EngineKey;
    middleTab?: string;
    instance?: string;
    view?: string;
}

export function buildEngineHash(
    engine: EngineKey,
    middleTab?: string,
    instance?: string,
    view?: string,
): string {
    const parts = ['#', 'engine', engine];
    if (middleTab) parts.push(middleTab);
    if (instance) { parts.push('instance'); parts.push(instance); }
    if (view) { parts.push('view'); parts.push(view); }
    return parts.join('/');
}

export function parseEngineHash(hash: string): RouteParams | null {
    const raw = hash.replace(/^#\/?/, '');
    if (!raw) return null;

    const segments = raw.split('/').filter(Boolean);
    if (segments.length < 2 || segments[0] !== 'engine') return null;

    const engine = segments[1] as EngineKey;
    const params: RouteParams = { engine };
    let i = 2;

    while (i < segments.length) {
        const key = segments[i];
        if (key === 'instance' && i + 1 < segments.length) {
            params.instance = segments[i + 1];
            i += 2;
        } else if (key === 'view' && i + 1 < segments.length) {
            params.view = segments[i + 1];
            i += 2;
        } else {
            // catch-all: treat as middleTab
            params.middleTab = key;
            i += 1;
        }
    }

    return params;
}

export function hashEquals(a: string, b: string): boolean {
    return a.replace(/^#\/?/, '') === b.replace(/^#\/?/, '');
}
