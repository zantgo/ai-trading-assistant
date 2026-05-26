//! # Separated WebSocket Stream & Telemetry Coordinator
//! 
//! Manages active server handshakes, formats numeric metric labels,
//! and aggregates raw high-resolution price ticks into moving candles.

const MAX_DATA_POINTS = 60;
const BAR_DURATION_MS = 5000; // 5-second candle updates

const statusEl = document.getElementById('connection-status');
const priceLbl = document.getElementById('price-lbl');
const ema10Lbl = document.getElementById('ema10-lbl');
const ema50Lbl = document.getElementById('ema50-lbl');
const ema100Lbl = document.getElementById('ema100-lbl');
const ema200Lbl = document.getElementById('ema200-lbl');

const macdLineLbl = document.getElementById('macd-line-lbl');
const macdSigLbl = document.getElementById('macd-sig-lbl');
const macdHistLbl = document.getElementById('macd-hist-lbl');

const rsiLbl = document.getElementById('rsi-lbl');
const sqzValLbl = document.getElementById('sqz-val-lbl');
const sqzStatusLbl = document.getElementById('sqz-status-lbl');
const adxLbl = document.getElementById('adx-lbl');

let lastMacdHist = 0;
let lastSqzMom = 0;

function connect() {
    const ws = new WebSocket(`ws://${window.location.host}/ws`);

    ws.onopen = () => {
        statusEl.innerHTML = `<span class="h-2 w-2 rounded-full bg-emerald-500 animate-pulse"></span><span>LIVE STREAM ACTIVE</span>`;
        statusEl.className = "px-3 py-1 rounded text-xs font-semibold bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 flex items-center space-x-2";
    };

    ws.onclose = () => {
        statusEl.innerHTML = `<span class="h-2 w-2 rounded-full bg-red-500 animate-pulse"></span><span>OFFLINE</span>`;
        statusEl.className = "px-3 py-1 rounded text-xs font-semibold bg-red-500/10 text-red-400 border border-red-500/20 flex items-center space-x-2";
        setTimeout(connect, 3000);
    };

    ws.onmessage = (event) => {
        const data = JSON.parse(event.data);
        
        // Convert to standard JS Date objects to guarantee timescale parsing
        const timestampMs = new Date(data.timestamp * 1000);
        const price = parseFloat(data.mid_price);

        // Update text labels
        priceLbl.innerText = `$${price.toFixed(2)}`;
        ema10Lbl.innerText = data.ema_10 ? parseFloat(data.ema_10).toFixed(2) : "--";
        ema50Lbl.innerText = data.ema_50 ? parseFloat(data.ema_50).toFixed(2) : "--";
        ema100Lbl.innerText = data.ema_100 ? parseFloat(data.ema_100).toFixed(2) : "--";
        ema200Lbl.innerText = data.ema_200 ? parseFloat(data.ema_200).toFixed(2) : "--";
        
        rsiLbl.innerText = data.rsi_14 ? parseFloat(data.rsi_14).toFixed(2) : "--";
        adxLbl.innerText = data.adx_14 ? parseFloat(data.adx_14).toFixed(2) : "--";

        // Aggregate ticks into 5-second candlesticks
        const roundedTime = new Date(Math.floor((data.timestamp * 1000) / BAR_DURATION_MS) * BAR_DURATION_MS);
        const candleData = priceChart.data.datasets[0].data;

        if (candleData.length > 0 && candleData[candleData.length - 1].x.getTime() === roundedTime.getTime()) {
            let lastCandle = candleData[candleData.length - 1];
            lastCandle.h = Math.max(lastCandle.h, price);
            lastCandle.l = Math.min(lastCandle.l, price);
            lastCandle.c = price;
        } else {
            candleData.push({
                x: roundedTime,
                o: price,
                h: price,
                l: price,
                c: price
            });
        }

        // Update EMA lines
        priceChart.data.datasets[1].data.push({ x: timestampMs, y: data.ema_10 ? parseFloat(data.ema_10) : null });
        priceChart.data.datasets[2].data.push({ x: timestampMs, y: data.ema_50 ? parseFloat(data.ema_50) : null });
        priceChart.data.datasets[3].data.push({ x: timestampMs, y: data.ema_100 ? parseFloat(data.ema_100) : null });
        priceChart.data.datasets[4].data.push({ x: timestampMs, y: data.ema_200 ? parseFloat(data.ema_200) : null });

        if (priceChart.data.datasets[0].data.length > MAX_DATA_POINTS) {
            priceChart.data.datasets[0].data.shift();
        }
        while (priceChart.data.datasets[1].data.length > MAX_DATA_POINTS * 5) {
            priceChart.data.datasets.slice(1).forEach(d => d.data.shift());
        }
        priceChart.update('none');

        // Update ADX Chart
        if (data.adx_14) {
            adxChart.data.datasets[0].data.push({ x: timestampMs, y: parseFloat(data.adx_14) });
            if (adxChart.data.datasets[0].data.length > MAX_DATA_POINTS * 5) {
                adxChart.data.datasets[0].data.shift();
            }
            adxChart.update('none');
        }

        // Update RSI Chart
        if (data.rsi_14) {
            rsiChart.data.datasets[0].data.push({ x: timestampMs, y: parseFloat(data.rsi_14) });
            if (rsiChart.data.datasets[0].data.length > MAX_DATA_POINTS * 5) {
                rsiChart.data.datasets[0].data.shift();
            }
            rsiChart.update('none');
        }

        // Update MACD Chart
        if (data.macd_line) {
            const mLine = parseFloat(data.macd_line);
            const mSig = parseFloat(data.macd_signal);
            const mHist = parseFloat(data.macd_hist);

            macdLineLbl.innerText = mLine.toFixed(2);
            macdSigLbl.innerText = mSig.toFixed(2);
            macdHistLbl.innerText = mHist.toFixed(2);

            macdChart.data.datasets[0].data.push({ x: timestampMs, y: mLine });
            macdChart.data.datasets[1].data.push({ x: timestampMs, y: mSig });
            macdChart.data.datasets[2].data.push({ x: timestampMs, y: mHist });

            let histColor = mHist >= 0 
                ? (mHist >= lastMacdHist ? "#26a69a" : "#b2dfdb")
                : (mHist < lastMacdHist ? "#ef5350" : "#ffcdd2");

            macdChart.data.datasets[2].backgroundColor.push(histColor);
            lastMacdHist = mHist;

            if (macdChart.data.datasets[0].data.length > MAX_DATA_POINTS * 5) {
                macdChart.data.datasets[0].data.shift();
                macdChart.data.datasets[1].data.shift();
                macdChart.data.datasets[2].data.shift();
                macdChart.data.datasets[2].backgroundColor.shift();
            }
            macdChart.update('none');
        }

        // Update Squeeze Chart
        if (data.squeeze_momentum) {
            const momVal = parseFloat(data.squeeze_momentum);
            sqzValLbl.innerText = momVal.toFixed(4);

            squeezeChart.data.datasets[0].data.push({ x: timestampMs, y: momVal });

            let momColor = momVal >= 0
                ? (momVal >= lastSqzMom ? "#4caf50" : "#086014")
                : (momVal < lastSqzMom ? "#ff1744" : "#800b1d");

            squeezeChart.data.datasets[0].backgroundColor.push(momColor);
            lastSqzMom = momVal;

            const isSqueezeOn = data.squeeze_on;
            squeezeChart.data.datasets[1].data.push({ x: timestampMs, y: 0 });
            
            if (isSqueezeOn) {
                sqzStatusLbl.innerHTML = "Status: <span class='text-red-500 font-bold'>SQUEEZE ON</span>";
                squeezeChart.data.datasets[1].pointBackgroundColor.push("#ef5350");
            } else {
                sqzStatusLbl.innerHTML = "Status: <span class='text-emerald-500 font-bold'>SQUEEZE OFF</span>";
                squeezeChart.data.datasets[1].pointBackgroundColor.push("#4caf50");
            }

            if (squeezeChart.data.datasets[0].data.length > MAX_DATA_POINTS * 5) {
                squeezeChart.data.datasets[0].data.shift();
                squeezeChart.data.datasets[0].backgroundColor.shift();
                squeezeChart.data.datasets[1].data.shift();
                squeezeChart.data.datasets[1].pointBackgroundColor.shift();
            }
            squeezeChart.update('none');
        }
    };
}

connect();
