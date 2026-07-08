// Adapter: flattens the nested `/api/history` indicator_history structure
// (v2.0 dual-representation) into the flat per-indicator string arrays the
// chart components consume (they parse them). Raw multi-line series are pulled
// from each indicator's `values` sub-map; single-line indicators use `raw`.

export interface FlatIndicatorHistory {
    times: number[];
    rsi_14: Array<string | null>;
    stoch_k: Array<string | null>;
    stoch_d: Array<string | null>;
    chandemo: Array<string | null>;
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
    avwap_weekly: Array<string | null>;
    avwap_monthly: Array<string | null>;
    avwap_swing: Array<string | null>;
    supertrend: Array<string | null>;
    keltner_upper: Array<string | null>;
    keltner_middle: Array<string | null>;
    keltner_lower: Array<string | null>;
    donchian_upper: Array<string | null>;
    donchian_middle: Array<string | null>;
    donchian_lower: Array<string | null>;
    ichimoku_tenkan: Array<string | null>;
    ichimoku_kijun: Array<string | null>;
    ichimoku_senkou_a: Array<string | null>;
    ichimoku_senkou_b: Array<string | null>;
    ichimoku_chikou: Array<string | null>;
    obv: Array<string | null>;
    cmf: Array<string | null>;
    mfi: Array<string | null>;
    hv: Array<string | null>;
    aroon: Array<string | null>;
    choppiness: Array<string | null>;
    linreg_slope: Array<string | null>;
    zscore: Array<string | null>;
    cci: Array<string | null>;
    psar_sar: Array<string | null>;
    williams_r: Array<string | null>;
    awesome_oscillator: Array<string | null>;
    force_index: Array<string | null>;
    hull_ma: Array<string | null>;
    stddev_upper: Array<string | null>;
    stddev_center: Array<string | null>;
    stddev_lower: Array<string | null>;
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
        stoch_k: val('stochastic', 'k_line'),
        stoch_d: val('stochastic', 'd_line'),
        chandemo: raw('chandemo'),
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
        avwap_weekly: val('anchored_vwap', 'weekly'),
        avwap_monthly: val('anchored_vwap', 'monthly'),
        avwap_swing: val('anchored_vwap', 'swing'),
        supertrend: val('supertrend', 'line'),
        keltner_upper: val('keltner', 'upper'),
        keltner_middle: val('keltner', 'middle'),
        keltner_lower: val('keltner', 'lower'),
        donchian_upper: val('donchian', 'upper'),
        donchian_middle: val('donchian', 'middle'),
        donchian_lower: val('donchian', 'lower'),
        ichimoku_tenkan: val('ichimoku', 'tenkan'),
        ichimoku_kijun: val('ichimoku', 'kijun'),
        ichimoku_senkou_a: val('ichimoku', 'senkou_a'),
        ichimoku_senkou_b: val('ichimoku', 'senkou_b'),
        ichimoku_chikou: val('ichimoku', 'chikou'),
        obv: raw('obv'),
        cmf: raw('cmf'),
        mfi: raw('mfi'),
        hv: raw('hv'),
        aroon: raw('aroon'),
        choppiness: raw('choppiness'),
        linreg_slope: raw('linreg_slope'),
        zscore: raw('zscore'),
        cci: raw('cci'),
        psar_sar: val('psar', 'sar'),
        williams_r: raw('williams_r'),
        awesome_oscillator: raw('awesome_oscillator'),
        force_index: raw('force_index'),
        hull_ma: raw('hull_ma'),
        stddev_upper: val('stddev_channel', 'upper'),
        stddev_center: val('stddev_channel', 'center'),
        stddev_lower: val('stddev_channel', 'lower'),
    };
}
