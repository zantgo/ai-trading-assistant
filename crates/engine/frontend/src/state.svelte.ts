// Global reactive state using Svelte 5 runes

let isConnected = $state(false);
let priceText = $state('--');
let emaFastText = $state('--');
let emaMediumText = $state('--');
let emaSlowText = $state('--');
let emaLongText = $state('--');
let adxText = $state('--');
let adxPlusText = $state('--');
let adxMinusText = $state('--');
let atrText = $state('--');
let rsiText = $state('--');
let macdLineText = $state('--');
let macdSigText = $state('--');
let macdHistText = $state('--');
let sqzValText = $state('--');
let sqzStatusText = $state('Calculating');
let isSqueezeOn = $state(false);
let volText = $state('--');
let vwapText = $state('--');

let activeSymbol = $state('ETH');
let candleTimeframeLabel = $state('5s');

let emaFastLabel = $state('EMA Fast');
let emaMediumLabel = $state('EMA Med');
let emaSlowLabel = $state('EMA Slow');
let emaLongLabel = $state('EMA Long');
let rsiLabel = $state('RSI (14)');
let adxLabel = $state('ADX (14)');
let atrLabel = $state('ATR (14)');
let macdLabel = $state('MACD (12, 26, 9)');

let showEmas = $state(true);
let showBb = $state(true);
let showVwap = $state(true);
let showVolume = $state(true);
let showAdx = $state(true);
let showAtr = $state(true);
let showRsi = $state(true);
let showMacd = $state(true);
let showSqueeze = $state(true);

let barDurationSec = $state(5);
let lastMacdHist = $state(0);
let lastSqzMom = $state(0);

let latestSnapshot: Record<string, unknown> | null = $state(null);

export function getState() {
    return {
        get isConnected() { return isConnected },
        set isConnected(v: boolean) { isConnected = v },
        get priceText() { return priceText },
        set priceText(v: string) { priceText = v },
        get emaFastText() { return emaFastText },
        set emaFastText(v: string) { emaFastText = v },
        get emaMediumText() { return emaMediumText },
        set emaMediumText(v: string) { emaMediumText = v },
        get emaSlowText() { return emaSlowText },
        set emaSlowText(v: string) { emaSlowText = v },
        get emaLongText() { return emaLongText },
        set emaLongText(v: string) { emaLongText = v },
        get adxText() { return adxText },
        set adxText(v: string) { adxText = v },
        get adxPlusText() { return adxPlusText },
        set adxPlusText(v: string) { adxPlusText = v },
        get adxMinusText() { return adxMinusText },
        set adxMinusText(v: string) { adxMinusText = v },
        get atrText() { return atrText },
        set atrText(v: string) { atrText = v },
        get rsiText() { return rsiText },
        set rsiText(v: string) { rsiText = v },
        get macdLineText() { return macdLineText },
        set macdLineText(v: string) { macdLineText = v },
        get macdSigText() { return macdSigText },
        set macdSigText(v: string) { macdSigText = v },
        get macdHistText() { return macdHistText },
        set macdHistText(v: string) { macdHistText = v },
        get sqzValText() { return sqzValText },
        set sqzValText(v: string) { sqzValText = v },
        get sqzStatusText() { return sqzStatusText },
        set sqzStatusText(v: string) { sqzStatusText = v },
        get isSqueezeOn() { return isSqueezeOn },
        set isSqueezeOn(v: boolean) { isSqueezeOn = v },
        get volText() { return volText },
        set volText(v: string) { volText = v },
        get vwapText() { return vwapText },
        set vwapText(v: string) { vwapText = v },
        get activeSymbol() { return activeSymbol },
        set activeSymbol(v: string) { activeSymbol = v },
        get candleTimeframeLabel() { return candleTimeframeLabel },
        set candleTimeframeLabel(v: string) { candleTimeframeLabel = v },
        get emaFastLabel() { return emaFastLabel },
        set emaFastLabel(v: string) { emaFastLabel = v },
        get emaMediumLabel() { return emaMediumLabel },
        set emaMediumLabel(v: string) { emaMediumLabel = v },
        get emaSlowLabel() { return emaSlowLabel },
        set emaSlowLabel(v: string) { emaSlowLabel = v },
        get emaLongLabel() { return emaLongLabel },
        set emaLongLabel(v: string) { emaLongLabel = v },
        get rsiLabel() { return rsiLabel },
        set rsiLabel(v: string) { rsiLabel = v },
        get adxLabel() { return adxLabel },
        set adxLabel(v: string) { adxLabel = v },
        get atrLabel() { return atrLabel },
        set atrLabel(v: string) { atrLabel = v },
        get macdLabel() { return macdLabel },
        set macdLabel(v: string) { macdLabel = v },
        get showEmas() { return showEmas },
        set showEmas(v: boolean) { showEmas = v },
        get showBb() { return showBb },
        set showBb(v: boolean) { showBb = v },
        get showVwap() { return showVwap },
        set showVwap(v: boolean) { showVwap = v },
        get showVolume() { return showVolume },
        set showVolume(v: boolean) { showVolume = v },
        get showAdx() { return showAdx },
        set showAdx(v: boolean) { showAdx = v },
        get showAtr() { return showAtr },
        set showAtr(v: boolean) { showAtr = v },
        get showRsi() { return showRsi },
        set showRsi(v: boolean) { showRsi = v },
        get showMacd() { return showMacd },
        set showMacd(v: boolean) { showMacd = v },
        get showSqueeze() { return showSqueeze },
        set showSqueeze(v: boolean) { showSqueeze = v },
        get barDurationSec() { return barDurationSec },
        set barDurationSec(v: number) { barDurationSec = v },
        get lastMacdHist() { return lastMacdHist },
        set lastMacdHist(v: number) { lastMacdHist = v },
        get lastSqzMom() { return lastSqzMom },
        set lastSqzMom(v: number) { lastSqzMom = v },
        get latestSnapshot() { return latestSnapshot },
        set latestSnapshot(v: Record<string, unknown> | null) { latestSnapshot = v },
    };
}
