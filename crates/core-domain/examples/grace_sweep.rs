//! Grace-band validation sweep (v6.10.16, institutional follow-up).
//!
//! Reads the periodic snapshot-export JSON corpus (06-03 / 08-09:
//! `<output_path>/<YYYY-MM-DD>/<HHhMMmSS>/<pair>.<slot>.<tab>.json`)
//! and measures — for every completed snapshot — which L3 bias rule
//! would have been directionally correct against the forward price
//! series of the same pair:
//!
//!   - plain ±20/±40 thresholds only,
//!   - the grace band swept over (band_min, vote ratio, agreement,
//!     signal breadth) with the v6.10.16 defaults as the centre point,
//!   - the shipped rule (grace + hysteresis) vs its no-hysteresis twin,
//!     comparing directional accuracy AND flip rate (Bullish↔Neutral↔
//!     Bearish transitions per sample).
//!
//! Usage:
//!   cargo run -p core-domain --example grace_sweep -- <snapshot_dir>
//!
//! The data path: enable `[snapshot_export]` in config.toml and let the
//! daemon run for a while; every tick writes one envelope per tab. When
//! no snapshots are found the tool prints a short usage note instead of
//! a fabricated result.

use core_domain::alignment::AlignmentMatrix;
use core_domain::analysis::{AnalysisMatrix, MarketBias};
use core_domain::snapshot_export::SnapshotEnvelope;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone)]
struct Sample {
    timestamp_ms: i64,
    symbol: String,
    price: f64,
    score: f64,
    agreement: f64,
    signals: u32,
    votes_bull: u32,
    votes_bear: u32,
    votes_flat: u32,
    engine_bias: MarketBias,
}

fn votes(alignment: &AlignmentMatrix) -> (u32, u32, u32) {
    let mut bull = 0u32;
    let mut bear = 0u32;
    let mut flat = 0u32;
    for tf in &alignment.timeframe_alignments {
        if tf.regime.to_uppercase() == "COMPRESSION" {
            continue;
        }
        if tf.overall_score > 10 {
            bull += 1;
        } else if tf.overall_score < -10 {
            bear += 1;
        } else {
            flat += 1;
        }
    }
    (bull, bear, flat)
}

/// Replicate the L3 bias decision for a swept constant set (no
/// hysteresis — the sweep isolates the band's intrinsic behaviour).
fn swept_bias(
    sample: &Sample,
    band_min: f64,
    vote_ratio: f64,
    agreement_min: f64,
    signals_min: u32,
) -> MarketBias {
    let score = sample.score;
    if score > 40.0 {
        return MarketBias::StrongBullish;
    }
    if score > 20.0 {
        return MarketBias::Bullish;
    }
    if score < -40.0 {
        return MarketBias::StrongBearish;
    }
    if score < -20.0 {
        return MarketBias::Bearish;
    }
    let required = ((sample.votes_bull + sample.votes_bear + sample.votes_flat) as f64 * vote_ratio)
        .ceil()
        .max(3.0) as u32;
    let fire_bull = score > band_min
        && score <= 20.0
        && sample.agreement >= agreement_min
        && sample.signals >= signals_min
        && sample.votes_bull >= required
        && sample.votes_bear <= 1;
    let fire_bear = score < -band_min
        && score >= -20.0
        && sample.agreement >= agreement_min
        && sample.signals >= signals_min
        && sample.votes_bear >= required
        && sample.votes_bull <= 1;
    if fire_bull {
        MarketBias::Bullish
    } else if fire_bear {
        MarketBias::Bearish
    } else {
        MarketBias::Neutral
    }
}

fn directional(score: f64) -> f64 {
    if score > 0.0 {
        1.0
    } else if score < 0.0 {
        -1.0
    } else {
        0.0
    }
}

struct ResultRow {
    label: String,
    directional_samples: usize,
    accuracy: f64,
    coverage_pct: f64,
    flips_per_sample: f64,
}

fn evaluate(
    samples: &[Sample],
    predict: impl Fn(&Sample) -> MarketBias,
    horizon: usize,
    with_flips: bool,
) -> ResultRow {
    let mut correct = 0usize;
    let mut directional_samples = 0usize;
    let mut flips = 0usize;
    let mut prev_dir: Option<f64> = None;
    for (i, s) in samples.iter().enumerate() {
        let pred = predict(s);
        let dir = match pred {
            MarketBias::Bullish | MarketBias::StrongBullish => 1.0,
            MarketBias::Bearish | MarketBias::StrongBearish => -1.0,
            MarketBias::Neutral => 0.0,
        };
        if with_flips && prev_dir.is_some() && dir != 0.0 && prev_dir != Some(dir) {
            flips += 1;
        }
        if dir != 0.0 {
            prev_dir = Some(dir);
            directional_samples += 1;
            let future = samples.get(i + horizon).map(|f| f.price).unwrap_or(s.price);
            let ret = directional(future - s.price);
            if ret != 0.0 && ret == dir {
                correct += 1;
            }
        }
    }
    ResultRow {
        label: String::new(),
        directional_samples,
        accuracy: if directional_samples > 0 {
            correct as f64 / directional_samples as f64
        } else {
            0.0
        },
        coverage_pct: if !samples.is_empty() {
            directional_samples as f64 / samples.len() as f64 * 100.0
        } else {
            0.0
        },
        flips_per_sample: if !samples.is_empty() {
            flips as f64 / samples.len() as f64
        } else {
            0.0
        },
    }
}

fn collect(root: &str) -> Vec<Sample> {
    // Two deterministic passes: alignment envelopes supply the cross-TF
    // matrix + price; analysis envelopes supply the engine bias for the
    // same (pair, timestamp_ms). Merging is keyed, so directory read
    // order can never decide which payload wins.
    let mut align: HashMap<(String, i64), (f64, AlignmentMatrix)> = HashMap::new();
    let mut bias: HashMap<(String, i64), MarketBias> = HashMap::new();
    let mut walk = vec![PathBuf::from(root)];
    while let Some(dir) = walk.pop() {
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for entry in rd.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    walk.push(p);
                } else if p.extension().is_some_and(|e| e == "json") {
                    let name = p.file_name().unwrap().to_string_lossy().to_string();
                    // Alignment payloads carry the cross-TF matrix; the
                    // analysis payload carries the engine bias for the
                    // same tick. Match on (timestamp_ms, pair).
                    if !name.contains(".alignment.json") && !name.contains(".analysis.json") {
                        continue;
                    }
                    let Ok(text) = std::fs::read_to_string(&p) else {
                        continue;
                    };
                    let Ok(envelope) = serde_json::from_str::<
                        SnapshotEnvelope<serde_json::Value>,
                    >(&text)
                    else {
                        continue;
                    };
                    let ts = envelope.snapshot_metadata.timestamp_ms;
                    let pair = envelope.snapshot_metadata.pair_key.clone();
                    let slot = name.contains(".alignment.json");
                    if slot {
                        if let Ok(alignment) =
                            serde_json::from_value::<AlignmentMatrix>(envelope.payload)
                        {
                            let price = alignment
                                .timeframe_alignments
                                .first()
                                .map(|tf| tf.price)
                                .unwrap_or(0.0);
                            align.insert((pair, ts), (price, alignment));
                        }
                    } else if let Ok(analysis) =
                        serde_json::from_value::<AnalysisMatrix>(envelope.payload)
                    {
                        bias.insert((pair, ts), analysis.bias);
                    }
                }
            }
        }
    }
    let mut out = Vec::new();
    let mut keys: Vec<&(String, i64)> = align.keys().collect();
    keys.sort_by_key(|k| k.1);
    for key in keys {
        let (price, alignment) = &align[key];
        let engine_bias = bias.get(key).copied().unwrap_or(MarketBias::Neutral);
        let (vb, vbe, vf) = votes(alignment);
        out.push(Sample {
            timestamp_ms: key.1,
            symbol: key.0.clone(),
            price: *price,
            score: alignment.mtf_overall_score,
            agreement: alignment.trend_agreement_pct,
            signals: alignment.signal_cross_tf_count,
            votes_bull: vb,
            votes_bear: vbe,
            votes_flat: vf,
            engine_bias,
        });
    }
    if std::env::var("GRACE_SWEEP_DEBUG").is_ok() {
        for s in &out {
            eprintln!(
                "DEBUG {} {} score={:.1} votes=({},{},{}) aggr={:.0} sig={} bias={:?} price={:.0}",
                s.symbol, s.timestamp_ms, s.score, s.votes_bull, s.votes_bear, s.votes_flat,
                s.agreement, s.signals, s.engine_bias, s.price
            );
        }
    }
    out
}

fn main() {
    let root = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: grace_sweep <snapshot_dir>");
        std::process::exit(2);
    });
    let samples = collect(&root);
    if samples.is_empty() {
        println!(
            "no snapshot envelopes found under `{}` — enable [snapshot_export] in config.toml \
             and let the daemon run; each tick writes one envelope per tab.",
            root
        );
        return;
    }
    let total = samples.len();
    let pairs: std::collections::HashSet<&str> =
        samples.iter().map(|s| s.symbol.as_str()).collect();
    println!(
        "corpus: {} samples across {} pair(s) — first {} UTCms, last {} UTCms\n",
        total,
        pairs.len(),
        samples.first().map(|s| s.timestamp_ms).unwrap_or(0),
        samples.last().map(|s| s.timestamp_ms).unwrap_or(0)
    );

    // Sweep the fire band (the user's 10–20 vs 15–20 question).
    let mut rows = Vec::new();
    for (label, band_min, vote_ratio, agreement, signals) in [
        ("plain ±20", 15.0, 0.75, 75.0, 3),
        ("band (15,20] · 3/4 · 75 · 3  [SHIPPED]", 15.0, 0.75, 75.0, 3),
        ("band (10,20] · 3/4 · 75 · 3", 10.0, 0.75, 75.0, 3),
        ("band (12,20] · 3/4 · 75 · 3", 12.0, 0.75, 75.0, 3),
        ("band (18,20] · 3/4 · 75 · 3", 18.0, 0.75, 75.0, 3),
        ("band (15,20] · 2/4 · 75 · 3", 15.0, 0.5, 75.0, 3),
        ("band (15,20] · 4/4 · 75 · 3", 15.0, 1.0, 75.0, 3),
        ("band (15,20] · 3/4 · 60 · 3", 15.0, 0.75, 60.0, 3),
        ("band (15,20] · 3/4 · 90 · 3", 15.0, 0.75, 90.0, 3),
        ("band (15,20] · 3/4 · 75 · 2", 15.0, 0.75, 75.0, 2),
    ] {
        let mut r = evaluate(&samples, |s| swept_bias(s, band_min, vote_ratio, agreement, signals), 1, false);
        r.label = label.to_string();
        rows.push(r);
    }
    // Plain-threshold-only accuracy for reference (band 20.01 → grace
    // can never fire; only the ±20/±40 thresholds act).
    let mut r = evaluate(&samples, |s| swept_bias(s, 20.01, 0.75, 75.0, 3), 1, false);
    r.label = "plain ±20/±40 only".to_string();
    rows.push(r);

    println!(
        "{:<44} {:>6} {:>8} {:>8} {:>10}",
        "rule", "dirN", "acc@1", "cov%", "flips/s"
    );
    for row in &rows {
        println!(
            "{:<44} {:>6} {:>7.1}% {:>7.1}% {:>10.3}",
            row.label,
            row.directional_samples,
            row.accuracy * 100.0,
            row.coverage_pct,
            row.flips_per_sample
        );
    }

    // Horizon sensitivity on the shipped fire rule.
    println!("\nshipped rule (15,20]·3/4·75·3 — horizon sensitivity:");
    for h in [1usize, 3, 6, 12] {
        let r = evaluate(
            &samples,
            |s| swept_bias(s, 15.0, 0.75, 75.0, 3),
            h,
            false,
        );
        println!("  horizon {} samples: acc {:.1}% over {} directional samples", h, r.accuracy * 100.0, r.directional_samples);
    }

    // Flip rate: shipped-with-hysteresis vs no-hysteresis (approximation:
    // the engine bias already embeds hysteresis; the swept rule does not).
    let engine = evaluate(&samples, |s| s.engine_bias, 1, true);
    let no_hyst = evaluate(&samples, |s| swept_bias(s, 15.0, 0.75, 75.0, 3), 1, true);
    println!(
        "\nflip rate (directional sign changes per sample): engine(w/ hysteresis) {:.4} vs no-hysteresis {:.4}",
        engine.flips_per_sample, no_hyst.flips_per_sample
    );
    println!(
        "engine directional accuracy @1: {:.1}% over {} samples",
        engine.accuracy * 100.0,
        engine.directional_samples
    );
}
