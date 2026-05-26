//! # Separated Technical Charts Initialization
//! 
//! Configures each of the 5 separate canvas chart instances.
//! Declares explicit global "type" fields to satisfy Chart.js v4 requirements,
//! and forces a fixed 65-pixel Y-axis width to align all grid borders perfectly.

const layoutConfig = {
    padding: { left: 10, right: 65, top: 15, bottom: 5 }
};

const gridConfig = {
    color: '#1c2030',
    borderColor: '#2a2e39',
    drawTicks: false
};

// Strict dot-decimal formatting callback logic overriding locale commas
const strictTickFormatter = {
    color: '#8f929d',
    font: { size: 9 },
    align: 'center',
    callback: function(value) {
        return Number(value).toFixed(1);
    }
};

// Shared X-Axis Timeline Configuration across upper charts
const sharedXScale = {
    type: 'time',
    time: {
        unit: 'second',
        displayFormats: { second: 'HH:mm:ss' }
    },
    grid: { display: false },
    ticks: { display: false }
};

// Force a static width on the Y-Axis to align all chart grid borders perfectly
const forceAlignYAxis = {
    afterFit: (scale) => {
        scale.width = 65; 
    }
};

// 1. Price & EMA Mixed Candlestick Chart (Global Type: 'candlestick')
const priceChart = new Chart(document.getElementById('priceChart').getContext('2d'), {
    type: 'candlestick', // Explicit global chart type
    data: {
        datasets: [
            {
                type: 'candlestick',
                label: 'Price',
                data: [],
                color: { up: '#26a69a', down: '#ef5350', unchanged: '#8f929d' },
                borderColor: { up: '#26a69a', down: '#ef5350', unchanged: '#8f929d' },
                wickColor: { up: '#26a69a', down: '#ef5350', unchanged: '#8f929d' }
            },
            { type: 'line', label: 'EMA 10', data: [], borderColor: '#2962ff', borderWidth: 1.5, pointRadius: 0, tension: 0.05 },
            { type: 'line', label: 'EMA 50', data: [], borderColor: '#ff9800', borderWidth: 1.5, pointRadius: 0, tension: 0.05 },
            { type: 'line', label: 'EMA 100', data: [], borderColor: '#e91e63', borderWidth: 1.5, pointRadius: 0, tension: 0.05 },
            { type: 'line', label: 'EMA 200', data: [], borderColor: '#9c27b0', borderWidth: 1.5, pointRadius: 0, tension: 0.05 },
        ]
    },
    options: {
        responsive: true,
        maintainAspectRatio: false,
        layout: layoutConfig,
        scales: {
            x: sharedXScale,
            y: { 
                position: 'right', 
                grid: gridConfig, 
                ticks: strictTickFormatter,
                suggestedMin: 1500,
                suggestedMax: 3500,
                ...forceAlignYAxis
            }
        },
        plugins: { legend: { display: false } }
    }
});

// 2. ADX Chart (Global Type: 'line')
const adxChart = new Chart(document.getElementById('adxChart').getContext('2d'), {
    type: 'line', // Explicit global chart type
    data: {
        datasets: [
            { label: 'ADX', data: [], borderColor: '#f1c40f', borderWidth: 1.5, pointRadius: 0, tension: 0.1 }
        ]
    },
    options: {
        responsive: true,
        maintainAspectRatio: false,
        layout: layoutConfig,
        scales: {
            x: sharedXScale,
            y: { 
                position: 'right', 
                grid: gridConfig, 
                ticks: strictTickFormatter,
                suggestedMin: 0,
                suggestedMax: 50,
                ...forceAlignYAxis
            }
        },
        plugins: { legend: { display: false } }
    }
});

// 3. RSI Chart (Global Type: 'line')
const rsiChart = new Chart(document.getElementById('rsiChart').getContext('2d'), {
    type: 'line', // Explicit global chart type
    data: {
        datasets: [
            { label: 'RSI', data: [], borderColor: '#7e57c2', borderWidth: 1.5, pointRadius: 0, tension: 0.1 }
        ]
    },
    options: {
        responsive: true,
        maintainAspectRatio: false,
        layout: layoutConfig,
        scales: {
            x: sharedXScale,
            y: { 
                position: 'right', 
                min: 0, max: 100,
                grid: gridConfig, 
                ticks: {
                    color: '#8f929d',
                    font: { size: 9 },
                    values: [30, 50, 70]
                },
                ...forceAlignYAxis
            }
        },
        plugins: { legend: { display: false } }
    }
});

// 4. MACD Chart (Global Type: 'line')
const macdChart = new Chart(document.getElementById('macdChart').getContext('2d'), {
    type: 'line', // Explicit global chart type
    data: {
        datasets: [
            { type: 'line', label: 'MACD', data: [], borderColor: '#2962ff', borderWidth: 1.5, pointRadius: 0, tension: 0.1 },
            { type: 'line', label: 'Signal', data: [], borderColor: '#ff9800', borderWidth: 1.5, pointRadius: 0, tension: 0.1 },
            { type: 'bar', label: 'Histogram', data: [], backgroundColor: [], barPercentage: 0.8, categoryPercentage: 0.8 }
        ]
    },
    options: {
        responsive: true,
        maintainAspectRatio: false,
        layout: layoutConfig,
        scales: {
            x: sharedXScale,
            y: { 
                position: 'right', 
                grid: gridConfig, 
                ticks: strictTickFormatter,
                suggestedMin: -5,
                suggestedMax: 5,
                ...forceAlignYAxis
            }
        },
        plugins: { legend: { display: false } }
    }
});

// 5. Squeeze Chart (Global Type: 'line')
const squeezeChart = new Chart(document.getElementById('squeezeChart').getContext('2d'), {
    type: 'line', // Explicit global chart type
    data: {
        datasets: [
            { type: 'bar', label: 'Momentum', data: [], backgroundColor: [], barPercentage: 0.8, categoryPercentage: 0.8 },
            { type: 'scatter', label: 'Squeeze Dot', data: [], pointBackgroundColor: [], pointRadius: 4, pointHoverRadius: 4 }
        ]
    },
    options: {
        responsive: true,
        maintainAspectRatio: false,
        layout: { padding: { left: 10, right: 65, top: 15, bottom: 20 } },
        scales: {
            x: { 
                type: 'time',
                time: {
                    unit: 'second',
                    displayFormats: { second: 'HH:mm:ss' }
                },
                grid: { color: '#1c2030', drawTicks: false }, 
                ticks: { color: '#64748b', font: { size: 9 }, maxRotation: 0 }
            },
            y: { 
                position: 'right', 
                grid: gridConfig, 
                ticks: strictTickFormatter,
                suggestedMin: -10,
                suggestedMax: 10,
                ...forceAlignYAxis
            }
        },
        plugins: { legend: { display: false } }
    }
});
