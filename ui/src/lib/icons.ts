const SVG_ATTRS = 'width="%s" height="%s" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"';

type IconSize = 'sm' | 'md' | 'lg' | number;

const sizeMap: Record<string, [number, number]> = {
    sm: [12, 12],
    md: [15, 15],
    lg: [20, 20],
};

function attrs(size: IconSize, extra?: string): string {
    let [w, h] = typeof size === 'number' ? [size, size] : (sizeMap[size] || sizeMap.md);
    return SVG_ATTRS.replace('%s', String(w)).replace('%s', String(h)) + (extra ? ' ' + extra : '');
}

function icon(body: string, size: IconSize = 'md', extra?: string): string {
    return `<svg ${attrs(size, extra)}>${body}</svg>`;
}

export const icons: Record<string, (size?: IconSize) => string> = {
    home:               (s) => icon('<path d="M3 10a2 2 0 0 1 .71-1.53l7-5.6a2 2 0 0 1 2.58 0l7 5.6A2 2 0 0 1 21 10v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/><polyline points="9 21 9 12 15 12 15 21"/>', s),
    database:           (s) => icon('<ellipse cx="12" cy="5" rx="9" ry="3"/><path d="M21 12c0 1.66-4 3-9 3S3 13.66 3 12"/><path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5"/>', s),
    trend:              (s) => icon('<polyline points="3 17 9 11 13 15 21 7"/><polyline points="15 7 21 7 21 13"/>', s),
    cycle:              (s) => icon('<path d="M3 4v5h5"/><path d="M3.5 15.5a9 9 0 0 0 15.8-2.5"/><path d="M21 20v-5h-5"/><path d="M20.5 8.5a9 9 0 0 0-15.8 2.5"/>', s),
    dollar:             (s) => icon('<rect x="2" y="3" width="20" height="18" rx="2"/><line x1="12" y1="8" x2="12" y2="16"/><path d="M9 10h3.5a1.5 1.5 0 0 1 0 3H9"/><line x1="12" y1="7" x2="12" y2="9"/><line x1="12" y1="15" x2="12" y2="17"/>', s),
    search:             (s) => icon('<circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/><path d="M15 11a4 4 0 1 1-8 0 4 4 0 0 1 8 0z"/>', s),
    key:                (s) => icon('<path d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4"/>', s),
    grid:               (s) => icon('<rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/>', s),
    logout:             (s) => icon('<path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"/><polyline points="16 17 21 12 16 7"/><line x1="21" y1="12" x2="9" y2="12"/>', s),
    menu:               (s) => icon('<line x1="3" y1="6" x2="21" y2="6"/><line x1="3" y1="12" x2="21" y2="12"/><line x1="3" y1="18" x2="21" y2="18"/>', s),
    chevronRight:       (s) => icon('<polyline points="9 18 15 12 9 6"/>', s, 'stroke-width="2"'),
    chevronDown:        (s) => icon('<polyline points="6 9 12 15 18 9"/>', s, 'stroke-width="2"'),
    x:                  (s) => icon('<line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>', s, 'stroke-width="1.8"'),
    play:               (s) => icon('<polygon points="6 3 20 12 6 21 6 3" fill="currentColor" stroke="none"/>', s),
    flask:              (s) => icon('<path d="M10 2v7.5L4.5 19a2 2 0 0 0 1.7 3h11.6a2 2 0 0 0 1.7-3L14 9.5V2"/><path d="M8.5 2h7"/><path d="M7 15h10"/>', s),
    pause:              (s) => icon('<rect x="5" y="3" width="6" height="18" rx="1" fill="currentColor" stroke="none"/><rect x="13" y="3" width="6" height="18" rx="1" fill="currentColor" stroke="none"/>', s),
    stop:               (s) => icon('<rect x="4" y="4" width="16" height="16" rx="2" fill="currentColor" stroke="none"/>', s),
    trash:              (s) => icon('<polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/><line x1="10" y1="11" x2="10" y2="17"/><line x1="14" y1="11" x2="14" y2="17"/>', s),
    info:               (s) => icon('<circle cx="12" cy="12" r="10"/><line x1="12" y1="16" x2="12" y2="12"/><line x1="12" y1="8" x2="12.01" y2="8"/>', s),
    settings:           (s) => icon('<circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>', s),
    upload:             (s) => icon('<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="17 8 12 3 7 8"/><line x1="12" y1="3" x2="12" y2="15"/>', s),
    activity:           (s) => icon('<polyline points="22 12 18 12 15 21 9 3 6 12 2 12"/>', s),
    shield:             (s) => icon('<path d="M12 2l8 4v6c0 5.55-3.84 10.74-8 12-4.16-1.26-8-6.45-8-12V6l8-4z"/>', s),
    layoutDashboard:    (s) => icon('<rect x="3" y="3" width="7" height="9" rx="1"/><rect x="14" y="3" width="7" height="5" rx="1"/><rect x="14" y="12" width="7" height="9" rx="1"/><rect x="3" y="16" width="7" height="5" rx="1"/>', s),
    tableChart:         (s) => icon('<rect x="3" y="3" width="18" height="18" rx="2"/><line x1="3" y1="9" x2="21" y2="9"/><line x1="3" y1="15" x2="21" y2="15"/><line x1="9" y1="3" x2="9" y2="21"/>', s),
};

export function getIcon(name: string, size: IconSize = 'md'): string {
    const fn = icons[name];
    return fn ? fn(size) : '';
}
