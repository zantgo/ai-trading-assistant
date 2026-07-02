// Adapter: flattens the nested `/api/history` indicator_history structure
// (v2.0 dual-representation) into the flat per-indicator string arrays the
// chart components consume (they parse them). Raw multi-line series are pulled
// from each indicator's `values` sub-map; single-line indicators use `raw`.

export interface FlatIndicatorHistory {
    times: number[];
    rsi_14: Array<string | null>;
    macd_line: Array<string | null>;
    macd_signal: Array<string | null>;
    macd_hist: Array<string | null>;
    adx_14: Array<string | null>;
    adx_plus: Array<string | null>;
    adx_minus: Array<string | null>;
    atr_14: Array<string | null>;
    bbwp: Array<string | null>;
    rvol: Array<string | null>;
    squeeze_momentum: Array<string | null>;
    squeeze_on: Array<boolean>;
    ema_fast: Array<string | null>;
    ema_medium: Array<string | null>;
    ema_slow: Array<string | null>;
    ema_long: Array<string | null>;
    bb_upper: Array<string | null>;
    bb_middle: Array<string | null>;
    bb_lower: Array<string | null>;
    vwap: Array<string | null>;
}

type NestedHistory = {
    times?: number[];
    indicators?: Record<
        string,
        {
            raw?: Array<number | null>;
            normalized?: Array<number | null>;
            state_label?: Array<string | null>;
            values?: Record<string, Array<number | null>>;
        }
    >;
};

const toStr = (arr: Array<number | null>): Array<string | null> =>
    arr.map((v) => (v == null ? null : String(v)));

export function flattenHistory(ih: NestedHistory | undefined | null): FlatIndicatorHistory {
    const map = ih?.indicators ?? {};
    const raw = (k: string): Array<string | null> => toStr(map[k]?.raw ?? []);
    const val = (k: string, s: string): Array<string | null> => toStr(map[k]?.values?.[s] ?? []);
    const label = (k: string): Array<string | null> => map[k]?.state_label ?? [];
    const adxMain = val('adx', 'adx');

    return {
        times: ih?.times ?? [],
        rsi_14: raw('rsi'),
        macd_line: val('macd', 'line'),
        macd_signal: val('macd', 'signal'),
        macd_hist: val('macd', 'histogram'),
        adx_14: adxMain.length ? adxMain : raw('adx'),
        adx_plus: val('adx', 'plus_di'),
        adx_minus: val('adx', 'minus_di'),
        atr_14: raw('atr'),
        bbwp: raw('bbwp'),
        rvol: raw('rvol'),
        squeeze_momentum: raw('squeeze'),
        squeeze_on: label('squeeze').map((l) => l === 'COMPRESSION_COILING'),
        ema_fast: val('ema_stack', 'fast'),
        ema_medium: val('ema_stack', 'medium'),
        ema_slow: val('ema_stack', 'slow'),
        ema_long: val('ema_stack', 'long'),
        bb_upper: val('bollinger', 'upper'),
        bb_middle: val('bollinger', 'middle'),
        bb_lower: val('bollinger', 'lower'),
        vwap: val('vwap', 'vwap'),
    };
}
