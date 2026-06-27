import type { AppStore } from '../state.svelte';
import type { TimeframeTelemetry } from '../types';

export type WsKey = 'wsMicro' | 'wsFast' | 'wsSlow' | 'wsMacro';

export interface WsState {
    wsMicro: WebSocket | null;
    wsFast: WebSocket | null;
    wsSlow: WebSocket | null;
    wsMacro: WebSocket | null;
    currentWsSymbol: string;
}

export function createWsState(): WsState {
    return {
        wsMicro: null,
        wsFast: null,
        wsSlow: null,
        wsMacro: null,
        currentWsSymbol: '',
    };
}

export function buildWsUrl(symbol: string, timeframeSecs: number): string {
    if (!symbol) return '';
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    return `${protocol}//${window.location.host}/ws?symbol=${encodeURIComponent(symbol)}&timeframe_secs=${timeframeSecs}`;
}

export function closeWs(ws: WebSocket | null): void {
    if (ws && ws.readyState !== WebSocket.CLOSED && ws.readyState !== WebSocket.CLOSING) {
        try { ws.onclose = null; ws.onerror = null; ws.close(); } catch (_) {}
    }
}

export function disconnectAllWs(state: WsState): void {
    closeWs(state.wsMicro); state.wsMicro = null;
    closeWs(state.wsFast); state.wsFast = null;
    closeWs(state.wsSlow); state.wsSlow = null;
    closeWs(state.wsMacro); state.wsMacro = null;
}

/** Parse and apply a WebSocket message to a TimeframeTelemetry object. */
export function applySnapshotToTimeframe(tf: TimeframeTelemetry, event: MessageEvent): void {
    try {
        const raw = JSON.parse(event.data);
        const snapshot = (raw.jsonrpc === '2.0' && raw.method === 'broadcast.market_snapshot')
            ? (raw.params?.snapshot || raw)
            : raw;
        if (!snapshot || typeof snapshot !== 'object') return;
        if (snapshot.mid_price !== undefined && snapshot.mid_price !== null) tf.priceText = parseFloat(snapshot.mid_price).toFixed(2);
        if (snapshot.vwap !== undefined && snapshot.vwap !== null) tf.vwapText = parseFloat(snapshot.vwap).toFixed(2);
        if (snapshot.vwap_bias != null) tf.vwapBias = String(snapshot.vwap_bias) as TimeframeTelemetry['vwapBias'];
        if (snapshot.ema_fast !== undefined && snapshot.ema_fast !== null) tf.emaFastText = parseFloat(snapshot.ema_fast).toFixed(2);
        if (snapshot.ema_medium !== undefined && snapshot.ema_medium !== null) tf.emaMediumText = parseFloat(snapshot.ema_medium).toFixed(2);
        if (snapshot.ema_slow !== undefined && snapshot.ema_slow !== null) tf.emaSlowText = parseFloat(snapshot.ema_slow).toFixed(2);
        if (snapshot.ema_long !== undefined && snapshot.ema_long !== null) tf.emaLongText = parseFloat(snapshot.ema_long).toFixed(2);
        if (snapshot.ema_stack_state != null) tf.emaStackState = String(snapshot.ema_stack_state) as TimeframeTelemetry['emaStackState'];
        if (snapshot.adx_14 !== undefined && snapshot.adx_14 !== null) tf.adxText = parseFloat(snapshot.adx_14).toFixed(2);
        if (snapshot.adx_plus !== undefined && snapshot.adx_plus !== null) tf.adxPlusText = parseFloat(snapshot.adx_plus).toFixed(2);
        if (snapshot.adx_minus !== undefined && snapshot.adx_minus !== null) tf.adxMinusText = parseFloat(snapshot.adx_minus).toFixed(2);
        if (snapshot.atr_14 !== undefined && snapshot.atr_14 !== null) tf.atrText = parseFloat(snapshot.atr_14).toFixed(2);
        if (snapshot.rsi_14 !== undefined && snapshot.rsi_14 !== null) tf.rsiText = parseFloat(snapshot.rsi_14).toFixed(2);
        if (snapshot.macd_line !== undefined && snapshot.macd_line !== null) tf.macdLineText = parseFloat(snapshot.macd_line).toFixed(4);
        if (snapshot.macd_signal !== undefined && snapshot.macd_signal !== null) tf.macdSigText = parseFloat(snapshot.macd_signal).toFixed(4);
        if (snapshot.macd_hist !== undefined && snapshot.macd_hist !== null) tf.macdHistText = parseFloat(snapshot.macd_hist).toFixed(4);
        if (snapshot.squeeze_momentum !== undefined && snapshot.squeeze_momentum !== null) tf.sqzValText = parseFloat(snapshot.squeeze_momentum).toFixed(4);
        if (snapshot.bbwp != null) {
            tf.bbwpText = parseFloat(String(snapshot.bbwp)).toFixed(1);
            tf.lastBbwp = parseFloat(String(snapshot.bbwp));
        }
        if (snapshot.chart_pattern != null) tf.activePattern = String(snapshot.chart_pattern) as TimeframeTelemetry['activePattern'];
        if (snapshot.chart_pattern_confidence != null) tf.patternConfidence = parseFloat(String(snapshot.chart_pattern_confidence));
        tf.isSqueezeOn = snapshot.squeeze_on ?? false;
        tf.sqzStatusText = tf.isSqueezeOn ? 'SQUEEZE ON' : 'SQUEEZE OFF';
        if (snapshot.volume !== undefined && snapshot.volume !== null) tf.volText = parseFloat(snapshot.volume).toFixed(2);
        if (snapshot.average_volume !== undefined && snapshot.average_volume !== null) tf.avgVolText = parseFloat(snapshot.average_volume).toFixed(2);
        tf.latestSnapshot = snapshot;

        tf.rsiDivergenceStatus = snapshot.rsi_divergence_status ? (snapshot.rsi_divergence_status as 'none' | 'potential' | 'confirmed') : 'none';
        tf.macdDivergenceStatus = snapshot.macd_divergence_status ? (snapshot.macd_divergence_status as 'none' | 'potential' | 'confirmed') : 'none';
        tf.rsiDivergenceCoords = snapshot.rsi_divergence_coords != null
            ? (typeof snapshot.rsi_divergence_coords === 'string' ? snapshot.rsi_divergence_coords : JSON.stringify(snapshot.rsi_divergence_coords))
            : null;
        tf.macdDivergenceCoords = snapshot.macd_divergence_coords != null
            ? (typeof snapshot.macd_divergence_coords === 'string' ? snapshot.macd_divergence_coords : JSON.stringify(snapshot.macd_divergence_coords))
            : null;

        tf.isCompleted = snapshot.is_completed === true;

        if (snapshot.macd_histogram_peak != null) tf.macdHistPeak = parseFloat(String(snapshot.macd_histogram_peak));
        if (snapshot.macd_crossover_detected != null) tf.macdCrossoverDetected = !!snapshot.macd_crossover_detected;
        if (snapshot.macd_crossover_direction != null) tf.macdCrossoverDirection = String(snapshot.macd_crossover_direction) as TimeframeTelemetry['macdCrossoverDirection'];
        if (snapshot.macd_trend_state != null) tf.macdContractionTriggered = snapshot.macd_trend_state === 'decelerating';

        if (snapshot.adx_regime != null) tf.adxTrendingRegime = String(snapshot.adx_regime) as TimeframeTelemetry['adxTrendingRegime'];
        if (snapshot.adx_di_crossover_detected != null) tf.adxDiCrossoverDetected = !!snapshot.adx_di_crossover_detected;
        if (snapshot.adx_di_crossover_direction != null) tf.adxDiCrossoverDirection = String(snapshot.adx_di_crossover_direction) as TimeframeTelemetry['adxDiCrossoverDirection'];
        if (snapshot.adx_slope != null) tf.adxSlope = parseFloat(String(snapshot.adx_slope));
        if (snapshot.adx_14 != null) tf.adxExhaustionReached = parseFloat(String(snapshot.adx_14)) > 40;

        if (snapshot.squeeze_duration != null) tf.squeezeDuration = Number(snapshot.squeeze_duration);
        if (snapshot.squeeze_release_trigger != null) tf.squeezeReleaseTrigger = !!snapshot.squeeze_release_trigger;
        if (snapshot.squeeze_momentum_direction != null) tf.squeezeMomentumDirection = String(snapshot.squeeze_momentum_direction) as TimeframeTelemetry['squeezeMomentumDirection'];

        if (snapshot.atr_volatility_regime != null) tf.atrVolatilityRegime = String(snapshot.atr_volatility_regime) as TimeframeTelemetry['atrVolatilityRegime'];
        if (snapshot.rvol != null) tf.rvol = parseFloat(String(snapshot.rvol));
    } catch (_) {}
}

export function connectWebsocketForTimeframe(
    app: AppStore,
    state: WsState,
    tf: TimeframeTelemetry,
    wsKey: WsKey,
    tfSecs: number,
): void {
    closeWs(state[wsKey]);

    const url = buildWsUrl(app.activeTab, tfSecs);
    if (!url) return;

    const newWs = new WebSocket(url);
    state[wsKey] = newWs;

    newWs.onopen = () => { app.isConnected = true; };
    newWs.onmessage = (event) => applySnapshotToTimeframe(tf, event);
    newWs.onclose = () => {
        app.isConnected = false;
        if (state[wsKey] === newWs) {
            state[wsKey] = null;
        }
        setTimeout(() => {
            if (app.activeTab === state.currentWsSymbol) {
                connectWebsocketForTimeframe(app, state, tf, wsKey, tfSecs);
            }
        }, 3000);
    };
    newWs.onerror = () => { newWs.close(); };
}

export function connectWebsocket(app: AppStore, state: WsState): void {
    const symbol = app.activeTab;
    if (!symbol) return;
    state.currentWsSymbol = symbol;

    const pair = app.instancesMap[symbol];
    if (!pair) return;

    connectWebsocketForTimeframe(app, state, pair.microTerm, 'wsMicro', pair.microTerm.barDurationSec);
    connectWebsocketForTimeframe(app, state, pair.fastTerm, 'wsFast', pair.fastTerm.barDurationSec);
    connectWebsocketForTimeframe(app, state, pair.slowTerm, 'wsSlow', pair.slowTerm.barDurationSec);
    connectWebsocketForTimeframe(app, state, pair.macroTerm, 'wsMacro', pair.macroTerm.barDurationSec);
}

/** Returns true if the active tab has changed since the last connect, meaning a reconnect is needed. */
export function shouldReconnect(app: AppStore, state: WsState): boolean {
    const tab = app.activeTab;
    return !!(tab && tab !== state.currentWsSymbol);
}
