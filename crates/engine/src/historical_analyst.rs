use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use crate::llm::{ChatMessage, ChatRequest, LlmClient, ResponseFormat};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalRecommendation {
    pub symbol: String,
    pub generated_at: String,
    pub trades_analyzed: usize,
    pub win_rate: f64,
    pub avg_risk_reward: f64,
    pub avg_hold_time_minutes: f64,
    pub profit_factor: f64,
    pub suggested_rr_adjustment: f64,
    pub suggested_position_sizing_pct: f64,
    pub regime_analysis: String,
    pub key_improvements: String,
    pub risk_recommendation: String,
}

pub struct HistoricalAnalyst {
    pub symbol: String,
    pub pair_key: String,
    pub pool: SqlitePool,
    pub llm_client: Arc<LlmClient>,
    pub sweep_interval_trades: u32,
    pub trade_counter: AtomicU32,
}

impl HistoricalAnalyst {
    pub fn new(
        symbol: String,
        pair_key: String,
        pool: SqlitePool,
        llm_client: Arc<LlmClient>,
        sweep_interval_trades: u32,
    ) -> Self {
        Self {
            symbol,
            pair_key,
            pool,
            llm_client,
            sweep_interval_trades,
            trade_counter: AtomicU32::new(0),
        }
    }

    /// Run the background analysis loop. Polls trade count and triggers analysis.
    pub async fn run_background_loop(&self, cancel: CancellationToken) {
        println!(
            "📊 Historical Analyst: Started for {} (every {} trades)",
            self.symbol, self.sweep_interval_trades
        );

        let mut last_analyzed_count: i64 = 0;

        loop {
            tokio::select! {
                biased;
                _ = cancel.cancelled() => {
                    println!("📊 Historical Analyst: {} cancelled", self.symbol);
                    break;
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(60)) => {}
            }

            // Check current trade count
            let current_count: i64 = match self.get_trade_count().await {
                Ok(c) => c,
                Err(e) => {
                    eprintln!(
                        "historical_analyst: get_trade_count failed for {}: {}",
                        self.symbol, e
                    );
                    continue;
                }
            };

            if current_count >= last_analyzed_count + self.sweep_interval_trades as i64 {
                if let Err(e) = self.run_analysis().await {
                    eprintln!(
                        "📊 Historical Analyst: {} analysis failed: {}",
                        self.symbol, e
                    );
                }
                last_analyzed_count = current_count;
            }
        }
    }

    async fn get_trade_count(&self) -> Result<i64, String> {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM trade_telemetry_history WHERE symbol = ?1")
                .bind(&self.symbol)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| format!("Failed to count trades: {}", e))?;
        Ok(row.0)
    }

    /// Run the full historical analysis cycle.
    pub async fn run_analysis(&self) -> Result<HistoricalRecommendation, String> {
        println!(
            "📊 Historical Analyst: Running analysis for {}...",
            self.symbol
        );

        // Fetch recent trades
        let trades: Vec<TradeRecord> = sqlx::query_as(
            "SELECT id, symbol, direction, entry_price, exit_price, realized_pnl, roi_percentage, \
             entry_timestamp, exit_timestamp, trigger_source \
             FROM trade_telemetry_history WHERE symbol = ?1 ORDER BY exit_timestamp DESC LIMIT 50",
        )
        .bind(&self.symbol)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to fetch trades: {}", e))?;

        if trades.is_empty() {
            return Err("No trades to analyze".into());
        }

        // Compute statistics
        let total = trades.len();
        let wins: Vec<&TradeRecord> = trades.iter().filter(|t| t.realized_pnl > 0.0).collect();
        let losses: Vec<&TradeRecord> = trades.iter().filter(|t| t.realized_pnl < 0.0).collect();

        let win_rate = wins.len() as f64 / total as f64;

        let avg_win: f64 = if !wins.is_empty() {
            wins.iter().map(|t| t.realized_pnl).sum::<f64>() / wins.len() as f64
        } else {
            0.0
        };

        let avg_loss: f64 = if !losses.is_empty() {
            losses.iter().map(|t| t.realized_pnl.abs()).sum::<f64>() / losses.len() as f64
        } else {
            0.0
        };

        let avg_risk_reward = if avg_loss > 0.0 {
            avg_win / avg_loss
        } else {
            0.0
        };

        let gross_profit: f64 = wins.iter().map(|t| t.realized_pnl).sum();
        let gross_loss: f64 = losses.iter().map(|t| t.realized_pnl.abs()).sum();
        let profit_factor = if gross_loss > 0.0 {
            gross_profit / gross_loss
        } else if gross_profit > 0.0 {
            f64::INFINITY
        } else {
            0.0
        };

        let avg_hold_time_minutes: f64 = trades
            .iter()
            .map(|t| {
                let dur = t.exit_timestamp.saturating_sub(t.entry_timestamp);
                dur as f64 / 60_000.0
            })
            .sum::<f64>()
            / total as f64;

        // Compute suggested adjustments
        let suggested_rr = if avg_risk_reward < 1.5 {
            (avg_risk_reward.max(1.0) + 0.5).min(3.0)
        } else {
            (avg_risk_reward * 0.9).max(2.0)
        };

        let suggested_sizing = if win_rate < 0.4 {
            1.0
        } else if win_rate < 0.5 {
            1.5
        } else {
            2.0
        };

        // Build trade summary for LLM
        let trade_summary = format!(
            "Total trades: {}\nWins: {} | Losses: {}\nWin Rate: {:.1}%\n\
             Avg Win: ${:.2} | Avg Loss: ${:.2}\n\
             Avg Risk/Reward: {:.2}\nProfit Factor: {:.2}\n\
             Avg Hold Time: {:.0} min",
            total,
            wins.len(),
            losses.len(),
            win_rate * 100.0,
            avg_win,
            avg_loss,
            avg_risk_reward,
            profit_factor,
            avg_hold_time_minutes,
        );

        // Call LLM for qualitative analysis
        let analysis = self.call_llm_analysis(&trade_summary, &trades).await?;

        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

        // Capture recommendations before moving analysis fields
        let regime_analysis = analysis.regime_analysis.clone();
        let key_improvements = analysis.key_improvements.clone();

        let recommendation = HistoricalRecommendation {
            symbol: self.symbol.clone(),
            generated_at: now,
            trades_analyzed: total,
            win_rate,
            avg_risk_reward,
            avg_hold_time_minutes,
            profit_factor,
            suggested_rr_adjustment: suggested_rr,
            suggested_position_sizing_pct: suggested_sizing,
            regime_analysis,
            key_improvements,
            risk_recommendation: analysis.risk_recommendation,
        };

        // Store in DB
        self.store_recommendation(&recommendation).await?;

        println!(
            "📊 Historical Analyst: {} complete — Win: {:.0}% PF: {:.2} R:R: {:.2}",
            self.symbol,
            win_rate * 100.0,
            profit_factor,
            avg_risk_reward
        );
        println!(
            "   Suggested R:R adjustment: {:.2}  |  Suggested sizing: {:.1}%",
            suggested_rr, suggested_sizing
        );
        if !analysis.key_improvements.is_empty() {
            println!("   Key improvements: {}", analysis.key_improvements);
        }
        if !analysis.regime_analysis.is_empty() {
            println!("   Regime analysis: {}", analysis.regime_analysis);
        }

        Ok(recommendation)
    }

    async fn call_llm_analysis(
        &self,
        trade_summary: &str,
        _trades: &[TradeRecord],
    ) -> Result<LlmAnalysisResult, String> {
        let system_prompt = HISTORICAL_ANALYST_PROMPT;

        let request_body = ChatRequest {
            model: self.llm_client.model.as_ref().clone(),
            messages: vec![
                ChatMessage {
                    role: "system".into(),
                    content: system_prompt.into(),
                },
                ChatMessage {
                    role: "user".into(),
                    content: trade_summary.to_string(),
                },
            ],
            temperature: 0.2,
            response_format: Some(ResponseFormat {
                format_type: "json_object".into(),
            }),
            max_tokens: 512,
        };

        let chat_response = self
            .llm_client
            .call_chat_completion(&request_body, "historical-analyst")
            .await?;

        let content = chat_response
            .choices
            .first()
            .ok_or("Historical analyst response had no choices")?
            .message
            .content
            .clone();

        let result: LlmAnalysisResult = serde_json::from_str(&content).map_err(|e| {
            format!(
                "Failed to parse historical analyst JSON: {}. Raw: {}",
                e, content
            )
        })?;

        Ok(result)
    }

    async fn store_recommendation(&self, rec: &HistoricalRecommendation) -> Result<(), String> {
        let key_improvements = &rec.key_improvements;
        let risk_recommendation = &rec.risk_recommendation;

        sqlx::query(
            "INSERT INTO historical_recommendations \
             (symbol, pair_key, generated_at, trades_analyzed, win_rate, avg_risk_reward, \
              avg_hold_time_minutes, profit_factor, suggested_rr, suggested_sizing_pct, \
              regime_analysis, key_improvements, risk_recommendation) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        )
        .bind(&rec.symbol)
        .bind(&self.pair_key)
        .bind(&rec.generated_at)
        .bind(rec.trades_analyzed as i64)
        .bind(rec.win_rate)
        .bind(rec.avg_risk_reward)
        .bind(rec.avg_hold_time_minutes)
        .bind(rec.profit_factor)
        .bind(rec.suggested_rr_adjustment)
        .bind(rec.suggested_position_sizing_pct)
        .bind(&rec.regime_analysis)
        .bind(key_improvements)
        .bind(risk_recommendation)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to store recommendation: {}", e))?;

        Ok(())
    }
}

// ─── LLM Response Schema ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LlmAnalysisResult {
    #[serde(default)]
    regime_analysis: String,
    #[serde(default)]
    key_improvements: String,
    #[serde(default)]
    risk_recommendation: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
#[allow(dead_code)]
struct TradeRecord {
    id: i64,
    symbol: String,
    direction: String,
    entry_price: f64,
    exit_price: f64,
    realized_pnl: f64,
    roi_percentage: f64,
    entry_timestamp: i64,
    exit_timestamp: i64,
    trigger_source: String,
}

// ─── System Prompt ─────────────────────────────────────────────────

const HISTORICAL_ANALYST_PROMPT: &str = r#"You are the Historical Trade Analyst Agent. Your task is to review the provided trade performance statistics and generate structured recommendations for improving future trading decisions.

Analyze the trade metrics and provide actionable insights. Focus on identifying patterns in losing trades and suggesting concrete adjustments.

OUTPUT strictly JSON, no markdown fences, no conversational preambles:
{
  "regime_analysis": "Brief analysis of which market regimes performed best/worst based on the trade data. 1-2 sentences.",
  "key_improvements": "Specific, actionable improvements to the trading strategy. Focus on entry timing, position sizing, or exit criteria. 1-2 sentences.",
  "risk_recommendation": "Suggested risk/reward target and position sizing adjustment. Include concrete numbers. 1-2 sentences."
}"#;

// ─── DB Schema Helper ─────────────────────────────────────────────

pub async fn add_historical_recommendations_table(pool: &SqlitePool) {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS historical_recommendations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            symbol TEXT NOT NULL,
            pair_key TEXT NOT NULL,
            generated_at TEXT NOT NULL,
            trades_analyzed INTEGER NOT NULL,
            win_rate REAL NOT NULL,
            avg_risk_reward REAL NOT NULL,
            avg_hold_time_minutes REAL NOT NULL,
            profit_factor REAL NOT NULL,
            suggested_rr REAL NOT NULL,
            suggested_sizing_pct REAL NOT NULL,
            regime_analysis TEXT NOT NULL DEFAULT '',
            key_improvements TEXT NOT NULL DEFAULT '',
            risk_recommendation TEXT NOT NULL DEFAULT ''
        )",
    )
    .execute(pool)
    .await
    .expect("Failed to create historical_recommendations table");

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_histrec_symbol ON historical_recommendations (symbol, generated_at DESC)"
    )
    .execute(pool)
    .await
    .ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_historical_recommendation_serialization() {
        let rec = HistoricalRecommendation {
            symbol: "BTC".into(),
            generated_at: "2025-01-01T00:00:00Z".into(),
            trades_analyzed: 50,
            win_rate: 0.55,
            avg_risk_reward: 2.1,
            avg_hold_time_minutes: 45.0,
            profit_factor: 1.8,
            suggested_rr_adjustment: 2.5,
            suggested_position_sizing_pct: 2.0,
            regime_analysis: "Strong trending performance".into(),
            key_improvements: "Wait for candle close".into(),
            risk_recommendation: "Target 2:1 minimum".into(),
        };
        let json = serde_json::to_string(&rec).unwrap();
        assert!(json.contains("win_rate"));
        assert!(json.contains("suggested_rr_adjustment"));
    }

    #[test]
    fn test_trade_record_from_row() {
        let rec = TradeRecord {
            id: 1,
            symbol: "BTC".into(),
            direction: "LONG".into(),
            entry_price: 50000.0,
            exit_price: 51000.0,
            realized_pnl: 1000.0,
            roi_percentage: 2.0,
            entry_timestamp: 1000,
            exit_timestamp: 2000,
            trigger_source: "AUTOMATED".into(),
        };
        assert!(rec.realized_pnl > 0.0);
        assert_eq!(rec.direction, "LONG");
    }
}
