use sqlx::SqlitePool;
use tokio_util::sync::CancellationToken;

use crate::db;

pub struct EvaluatorConfig {
    pub pool: SqlitePool,
    pub cancel: CancellationToken,
    pub eval_interval_secs: u64,
}

pub async fn run_performance_evaluator(cfg: EvaluatorConfig) {
    println!("📊 Performance Evaluator: Started (interval: {}s)...", cfg.eval_interval_secs);

    loop {
        tokio::select! {
            biased;
            _ = cfg.cancel.cancelled() => {
                println!("🛑 Performance Evaluator: Cancelled, shutting down.");
                break;
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(cfg.eval_interval_secs)) => {}
        }

        let entries = db::query_pending_performance_entries(&cfg.pool).await;
        if entries.is_empty() {
            continue;
        }

        let now_ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        for entry in &entries {
            let created_ts = match parse_sqlite_datetime(&entry.created_at) {
                Some(ts) => ts as i64,
                None => {
                    eprintln!("⚠️ PerfEval: Failed to parse created_at '{}' for entry {}", entry.created_at, entry.id);
                    continue;
                }
            };

            let signal_price: f64 = match entry.price_at_signal.parse() {
                Ok(v) => v,
                Err(_) => continue,
            };

            let action = db::query_master_action_by_id(&cfg.pool, entry.master_record_id).await;
            let action = match action {
                Some(a) => a,
                None => continue,
            };

            let mut price_1h: Option<String> = entry.price_at_1h.clone();
            let mut corr_1h: Option<bool> = entry.direction_correct_1h;
            let mut price_4h: Option<String> = entry.price_at_4h.clone();
            let mut corr_4h: Option<bool> = entry.direction_correct_4h;
            let mut price_24h: Option<String> = entry.price_at_24h.clone();
            let mut corr_24h: Option<bool> = entry.direction_correct_24h;

            // Horizon 1h
            if price_1h.is_none() {
                let target_ts = created_ts + 3600;
                if now_ts >= target_ts {
                    if let Some(px) = db::query_closest_close_price(&cfg.pool, &entry.symbol, 60, target_ts as u64).await {
                        price_1h = Some(px.to_string());
                        corr_1h = evaluate_direction(&action, signal_price, px);
                    }
                }
            }

            // Horizon 4h
            if price_4h.is_none() {
                let target_ts = created_ts + 14400;
                if now_ts >= target_ts {
                    if let Some(px) = db::query_closest_close_price(&cfg.pool, &entry.symbol, 60, target_ts as u64).await {
                        price_4h = Some(px.to_string());
                        corr_4h = evaluate_direction(&action, signal_price, px);
                    }
                }
            }

            // Horizon 24h
            if price_24h.is_none() {
                let target_ts = created_ts + 86400;
                if now_ts >= target_ts {
                    if let Some(px) = db::query_closest_close_price(&cfg.pool, &entry.symbol, 60, target_ts as u64).await {
                        price_24h = Some(px.to_string());
                        corr_24h = evaluate_direction(&action, signal_price, px);
                    }
                }
            }

            if price_1h != entry.price_at_1h
                || price_4h != entry.price_at_4h
                || price_24h != entry.price_at_24h
            {
                db::update_performance_tracker_prices(
                    &cfg.pool,
                    entry.id,
                    price_1h.as_deref(),
                    corr_1h,
                    price_4h.as_deref(),
                    corr_4h,
                    price_24h.as_deref(),
                    corr_24h,
                )
                .await;
            }
        }
    }

    println!("🛑 Performance Evaluator: Terminated.");
}

fn evaluate_direction(action: &str, signal_price: f64, horizon_price: f64) -> Option<bool> {
    match action {
        "Open Long" => Some(horizon_price > signal_price),
        "Open Short" => Some(horizon_price < signal_price),
        "Hold" | "Close" | "Wait" => None,
        _ => None,
    }
}

fn parse_sqlite_datetime(s: &str) -> Option<u64> {
    let parts: Vec<&str> = s.split(&[' ', '-', ':']).filter(|p| !p.is_empty()).collect();
    if parts.len() < 6 {
        return None;
    }
    let year: i32 = parts[0].parse().ok()?;
    let month: u32 = parts[1].parse().ok()?;
    let day: u32 = parts[2].parse().ok()?;
    let hour: u32 = parts[3].parse().ok()?;
    let min: u32 = parts[4].parse().ok()?;
    let sec: u32 = parts[5].parse().ok()?;

    let days_since_epoch = days_from_civil(year, month, day)?;
    let total_secs = days_since_epoch as u64 * 86400 + hour as u64 * 3600 + min as u64 * 60 + sec as u64;
    Some(total_secs)
}

fn days_from_civil(y: i32, m: u32, d: u32) -> Option<i64> {
    if m < 1 || m > 12 || d < 1 || d > 31 {
        return None;
    }
    let y = y as i64;
    let m = m as i64;
    let d = d as i64;
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y / 400 } else { (y - 399) / 400 };
    let yoe = (y - era * 400) as u64;
    let doy = if m <= 2 {
        (153 * (m + 9) + 2) / 5 + d - 1
    } else {
        (153 * (m - 3) + 2) / 5 + d - 1
    } as u64;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some((era * 146097 + doe as i64 - 719468) as i64)
}
