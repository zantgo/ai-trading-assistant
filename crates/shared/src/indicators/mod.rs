pub mod adx;
pub mod atr;
pub mod bbwp;
pub mod bollinger;
pub mod divergence;
pub mod ema;
pub mod fibonacci;
pub mod macd;
pub mod normalized;
pub mod patterns;
pub mod rsi;
pub mod sma;
pub mod squeeze;
pub mod traits;

pub use adx::{Adx, AdxOutput, DiCrossoverDir, TrendRegime};
pub use atr::{Atr, AtrOutput, VolatilityRegime};
pub use bbwp::Bbwp;
pub use bollinger::BollingerBands;
pub use divergence::{
    DivergenceCoords, DivergenceDetector, DivergenceResult, DivergenceStatus, DivergenceType,
    PeakTrough,
};
pub use ema::Ema;
pub use fibonacci::{FibonacciRange, PivotPoint, PivotType, SwingLegType};
pub use macd::{CrossoverDir, Macd, MacdOutput, TrendState};
pub use normalized::{
    DivergenceState, IndicatorInputs, NormalizationContext, NormalizationEngine,
    NormalizedIndicatorValue,
};
pub use patterns::{detect_pattern, ChartPattern, PatternResult};
pub use rsi::Rsi;
pub use sma::Sma;
pub use squeeze::{MomentumDirection, SqueezeMomentum, SqueezeOutput};
pub use traits::{BarInput, Indicator};
