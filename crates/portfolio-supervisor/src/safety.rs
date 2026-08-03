use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal_macros::dec;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use core_domain::portfolio::{SafetyState, VetoTrigger};

pub struct SafetyManager {
    pub consecutive_losses: RwLock<HashMap<String, u32>>,
    pub safety_state: RwLock<SafetyState>,
    pub dropout_until: RwLock<Option<Instant>>,
    pub dropout_symbol: RwLock<Option<String>>,
    pub initial_capital: RwLock<Decimal>,
    pub current_equity: RwLock<Decimal>,
    pub peak_equity: RwLock<Decimal>,
    pub starting_session_equity: RwLock<Decimal>,
    pub daily_pnl: RwLock<Decimal>,
    pub manual_stance: RwLock<Option<(String, String)>>,

    pub caution_threshold: u32,
    pub dropout_threshold: u32,
    pub dropout_duration: Duration,
    pub drawdown_limit_pct: Decimal,
    pub max_daily_drawdown_pct: f64,
    pub systemic_risk_threshold: f64,
    pub pool: RwLock<Option<Arc<sqlx::SqlitePool>>>,
}

impl SafetyManager {
    pub fn new(
        caution_threshold: u32,
        dropout_threshold: u32,
        dropout_duration_hours: u64,
        drawdown_limit_pct: f64,
        max_daily_drawdown_pct: f64,
        systemic_risk_threshold: f64,
    ) -> Self {
        Self {
            consecutive_losses: RwLock::new(HashMap::new()),
            safety_state: RwLock::new(SafetyState::Normal),
            dropout_until: RwLock::new(None),
            dropout_symbol: RwLock::new(None),
            initial_capital: RwLock::new(dec!(0)),
            current_equity: RwLock::new(dec!(0)),
            peak_equity: RwLock::new(dec!(0)),
            starting_session_equity: RwLock::new(dec!(0)),
            daily_pnl: RwLock::new(dec!(0)),
            manual_stance: RwLock::new(None),
            caution_threshold,
            dropout_threshold,
            dropout_duration: Duration::from_secs(dropout_duration_hours * 3600),
            drawdown_limit_pct: Decimal::from_f64_retain(drawdown_limit_pct).unwrap_or(dec!(30)),
            max_daily_drawdown_pct,
            systemic_risk_threshold,
            pool: RwLock::new(None),
        }
    }

    pub async fn set_db_pool(&self, pool: Arc<SqlitePool>) {
        *self.pool.write().await = Some(pool);
    }

    pub async fn set_manual_stance(&self, symbol: &str, stance: &str) {
        *self.manual_stance.write().await = Some((symbol.to_string(), stance.to_string()));
    }

    pub async fn clear_manual_stance(&self) {
        *self.manual_stance.write().await = None;
    }

    async fn persist_peak_equity(&self) {
        if let Some(ref pool) = *self.pool.read().await {
            let peak = *self.peak_equity.read().await;
            let peak_str = peak.to_string();
            let _ = sqlx::query(
                "UPDATE paper_balances SET peak_equity = ?1",
            )
            .bind(&peak_str)
            .execute(pool.as_ref())
            .await;
        }
    }

    pub async fn record_trade_outcome(&self, symbol: &str, is_loss: bool) -> SafetyState {
        let mut losses = self.consecutive_losses.write().await;

        if is_loss {
            let count = losses.entry(symbol.to_string()).and_modify(|c| *c += 1).or_insert(1);
            let current_count = *count;

            if current_count >= self.dropout_threshold {
                let until = Instant::now() + self.dropout_duration;
                *self.dropout_until.write().await = Some(until);
                *self.dropout_symbol.write().await = Some(symbol.to_string());
                *self.safety_state.write().await = SafetyState::Suspended;
                eprintln!(
                    "🛑 SAFETY: Symbol {} suspended for {}h after {} consecutive losses",
                    symbol,
                    self.dropout_duration.as_secs() / 3600,
                    current_count
                );
            } else if current_count >= self.caution_threshold {
                let current_state = *self.safety_state.read().await;
                if current_state != SafetyState::Suspended
                    && current_state != SafetyState::DrawdownStop
                {
                    *self.safety_state.write().await = SafetyState::Cautious;
                }
                eprintln!(
                    "⚠️  SAFETY: Symbol {} entering Cautious mode after {} consecutive losses",
                    symbol, current_count
                );
            }
        } else {
            losses.insert(symbol.to_string(), 0);
            let current_state = *self.safety_state.read().await;
            if current_state == SafetyState::Cautious || current_state == SafetyState::Suspended {
                let all_clear = losses.values().all(|c| *c < self.caution_threshold);
                if all_clear {
                    *self.safety_state.write().await = SafetyState::Normal;
                    *self.dropout_until.write().await = None;
                    *self.dropout_symbol.write().await = None;
                }
            }
        }

        *self.safety_state.read().await
    }

    pub async fn check_allow_trade(&self, symbol: &str) -> Result<(), String> {
        let state = *self.safety_state.read().await;

        match state {
            SafetyState::DrawdownStop => {
                return Err("Trading halted: capital drawdown limit exceeded".into());
            }
            SafetyState::Suspended => {
                let drop_symbol = self.dropout_symbol.read().await.clone();
                if drop_symbol.as_deref() == Some(symbol) {
                    if let Some(until) = *self.dropout_until.read().await {
                        if Instant::now() < until {
                            let remaining = until.duration_since(Instant::now()).as_secs();
                            return Err(format!(
                                "Trading suspended for {}: {}s remaining in dropout",
                                symbol, remaining
                            ));
                        }
                    }
                    *self.safety_state.write().await = SafetyState::Normal;
                    *self.dropout_until.write().await = None;
                    *self.dropout_symbol.write().await = None;
                    let mut losses = self.consecutive_losses.write().await;
                    losses.insert(symbol.to_string(), 0);
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub async fn evaluate_daily_drawdown_warn(&self) -> Option<SafetyState> {
        let daily_pnl = *self.daily_pnl.read().await;
        let start_equity = *self.starting_session_equity.read().await;
        let current_state = *self.safety_state.read().await;

        if current_state == SafetyState::DrawdownStop {
            return None;
        }

        if start_equity <= dec!(0) {
            return None;
        }

        let daily_drawdown = (-daily_pnl / start_equity * dec!(100))
            .to_f64()
            .unwrap_or(0.0);

        if daily_drawdown >= self.max_daily_drawdown_pct {
            if current_state != SafetyState::Warn {
                *self.safety_state.write().await = SafetyState::Warn;
                eprintln!(
                    "⚠️  SAFETY: WARN — daily drawdown {:.2}% exceeds {:.1}% limit",
                    daily_drawdown, self.max_daily_drawdown_pct
                );
            }
            return Some(SafetyState::Warn);
        }

        if current_state == SafetyState::Warn && daily_drawdown < self.max_daily_drawdown_pct {
            *self.safety_state.write().await = SafetyState::Normal;
            return Some(SafetyState::Normal);
        }

        None
    }

    pub async fn check_capital_drawdown(&self) -> Result<(), String> {
        let current = *self.current_equity.read().await;
        let peak = *self.peak_equity.read().await;

        if peak <= dec!(0) {
            return Ok(());
        }

        let ratio = current / peak;
        let limit_fraction = Decimal::ONE - (self.drawdown_limit_pct / dec!(100));

        if ratio < limit_fraction {
            *self.safety_state.write().await = SafetyState::DrawdownStop;
            let loss_pct = ((dec!(1) - ratio) * dec!(100)).to_f64().unwrap_or(0.0);
            eprintln!(
                "🛑 SAFETY: Drawdown stop triggered. Equity {:.1}% of peak exceeds {:.1}% limit",
                ratio.to_f64().unwrap_or(0.0) * 100.0,
                self.drawdown_limit_pct.to_f64().unwrap_or(30.0)
            );
            return Err(format!(
                "Capital drawdown {:.1}% exceeds {:.1}% limit",
                loss_pct,
                self.drawdown_limit_pct.to_f64().unwrap_or(30.0)
            ));
        }
        Ok(())
    }

    pub async fn update_peak_equity(&self, current: Decimal) {
        let mut peak = self.peak_equity.write().await;
        if current > *peak {
            *peak = current;
            drop(peak);
            self.persist_peak_equity().await;
        }
    }

    pub async fn evaluate_all(
        &self,
        symbol: &str,
        margin_usage_ratio: Decimal,
        systemic_risk_score: f64,
        exposure_breached: bool,
    ) -> Vec<VetoTrigger> {
        let mut triggers = Vec::new();

        self.update_peak_equity(*self.current_equity.read().await).await;

        // Priority 1: Drawdown breach (AVOID + Hard Exit)
        if self.check_capital_drawdown().await.is_err() {
            triggers.push(VetoTrigger {
                condition: "drawdown_breach".into(),
                target_stance: "AVOID".into(),
                reason: "Capital drawdown exceeds limit".into(),
                hard_exit: true,
            });
        }

        // Priority 2: Margin exhaustion (AVOID + Hard Exit)
        if margin_usage_ratio >= dec!(1) {
            triggers.push(VetoTrigger {
                condition: "margin_exhaustion".into(),
                target_stance: "AVOID".into(),
                reason: format!(
                    "Margin usage ratio {:.2} exceeds 1.00",
                    margin_usage_ratio
                ),
                hard_exit: true,
            });
        }

        // Priority 3: Systemic risk (AVOID + Hard Exit)
        if systemic_risk_score >= self.systemic_risk_threshold {
            triggers.push(VetoTrigger {
                condition: "systemic_risk".into(),
                target_stance: "AVOID".into(),
                reason: format!(
                    "Systemic risk score {:.1} >= {:.1}",
                    systemic_risk_score, self.systemic_risk_threshold
                ),
                hard_exit: true,
            });
        }

        // Priority 4: Margin ceiling (CLOSE_ONLY, no Hard Exit)
        if margin_usage_ratio >= dec!(0.95) && margin_usage_ratio < dec!(1) {
            triggers.push(VetoTrigger {
                condition: "margin_ceiling".into(),
                target_stance: "CLOSE_ONLY".into(),
                reason: format!(
                    "Margin usage ratio {:.2} >= 0.95",
                    margin_usage_ratio
                ),
                hard_exit: false,
            });
        }

        // Priority 5: Exposure limit breach (CLOSE_ONLY, no Hard Exit)
        if exposure_breached {
            triggers.push(VetoTrigger {
                condition: "exposure_limit_breach".into(),
                target_stance: "CLOSE_ONLY".into(),
                reason: "Portfolio exposure exceeds concentration limits".into(),
                hard_exit: false,
            });
        }

        // Priority 6: Loss streak (CLOSE_ONLY per-symbol, no Hard Exit)
        let losses = self.consecutive_losses.read().await;
        if let Some(&count) = losses.get(symbol) {
            if count >= self.dropout_threshold {
                triggers.push(VetoTrigger {
                    condition: "loss_streak".into(),
                    target_stance: "CLOSE_ONLY".into(),
                    reason: format!(
                        "Symbol {} has {} consecutive losses >= {}",
                        symbol, count, self.dropout_threshold
                    ),
                    hard_exit: false,
                });
            }
        }

        // Priority 7: Manual override (operator-initiated)
        let manual = self.manual_stance.read().await.clone();
        if let Some((manual_symbol, stance)) = manual {
            triggers.push(VetoTrigger {
                condition: "manual_override".into(),
                target_stance: stance.clone(),
                reason: format!(
                    "Operator-initiated override for {}: {}",
                    manual_symbol, stance
                ),
                hard_exit: stance == "AVOID",
            });
        }

        triggers
    }

    pub async fn reset_consecutive_losses(&self, symbol: Option<&str>) {
        let mut losses = self.consecutive_losses.write().await;
        match symbol {
            Some(s) => {
                losses.insert(s.to_string(), 0);
            }
            None => {
                losses.clear();
            }
        }
        let current_state = *self.safety_state.read().await;
        if current_state == SafetyState::Cautious || current_state == SafetyState::Suspended {
            let all_clear = losses.values().all(|c| *c < self.caution_threshold);
            if all_clear && current_state != SafetyState::DrawdownStop {
                *self.safety_state.write().await = SafetyState::Normal;
                *self.dropout_until.write().await = None;
                *self.dropout_symbol.write().await = None;
            }
        }
        println!("🔄 SAFETY: Loss counter reset");
    }

    pub async fn set_initial_capital(&self, capital: Decimal) {
        *self.initial_capital.write().await = capital;
        self.update_peak_equity(capital).await;
        let ss_equity = self.starting_session_equity.read().await;
        if *ss_equity == dec!(0) {
            drop(ss_equity);
            *self.starting_session_equity.write().await = capital;
        }
        self.persist_capital_state(capital).await;
    }

    pub async fn set_current_equity(&self, equity: Decimal) {
        *self.current_equity.write().await = equity;
        self.update_peak_equity(equity).await;
    }

    async fn persist_capital_state(&self, capital: Decimal) {
        if let Some(ref pool) = *self.pool.read().await {
            let capital_str = capital.to_string();
            let peak_str = self.peak_equity.read().await.to_string();
            let session_str = self.starting_session_equity.read().await.to_string();
            let _ = sqlx::query(
                "UPDATE paper_balances SET initial_usd = ?1, current_cash = ?1, peak_equity = ?2, starting_session_equity = ?3",
            )
            .bind(&capital_str)
            .bind(&peak_str)
            .bind(&session_str)
            .execute(pool.as_ref())
            .await;
        }
    }

    pub async fn session_reset(&self) {
        let current = *self.current_equity.read().await;
        *self.starting_session_equity.write().await = current;
        *self.peak_equity.write().await = current;
        *self.daily_pnl.write().await = dec!(0);

        let current_state = *self.safety_state.read().await;
        if current_state == SafetyState::Warn {
            *self.safety_state.write().await = SafetyState::Normal;
        }
        println!("🔄 SAFETY: Session reset — peak_equity and daily PnL re-baselined");
    }

    pub async fn release_veto(&self, reset_peak: bool) -> Result<(), String> {
        let state = *self.safety_state.read().await;

        match state {
            SafetyState::DrawdownStop => {
                self.check_capital_drawdown().await?;
                *self.safety_state.write().await = SafetyState::Normal;
                println!("🔓 SAFETY: Drawdown veto released");
            }
            SafetyState::Warn | SafetyState::Cautious | SafetyState::Suspended => {
                *self.safety_state.write().await = SafetyState::Normal;
                *self.dropout_until.write().await = None;
                *self.dropout_symbol.write().await = None;
                let mut losses = self.consecutive_losses.write().await;
                losses.clear();
                println!("🔓 SAFETY: Safety state reset to Normal");
            }
            SafetyState::Normal => {}
        }

        if reset_peak {
            let current = *self.current_equity.read().await;
            *self.peak_equity.write().await = current;
            self.persist_peak_equity().await;
            println!("🔓 SAFETY: Peak equity reset to current equity");
        }

        self.clear_manual_stance().await;

        Ok(())
    }

    pub async fn get_safety_context(&self) -> String {
        let state = *self.safety_state.read().await;
        let losses = self.consecutive_losses.read().await;
        let total_losses: u32 = losses.values().sum();
        match state {
            SafetyState::Normal => {
                format!("Normal risk mode. {} total consecutive losses across symbols.", total_losses)
            }
            SafetyState::Warn => format!(
                "WARN: Daily drawdown exceeds {}% limit. No stance change applied.",
                self.max_daily_drawdown_pct
            ),
            SafetyState::Cautious => format!(
                "CAUTION: {} total consecutive losses. Tighten entry criteria.",
                total_losses
            ),
            SafetyState::Suspended => {
                let remaining = self
                    .dropout_until
                    .read()
                    .await
                    .map(|u| {
                        u.duration_since(Instant::now())
                            .as_secs()
                    })
                    .unwrap_or(0);
                let sym = self.dropout_symbol.read().await.clone()
                    .unwrap_or_else(|| "unknown".into());
                format!(
                    "SUSPENDED: Symbol {} — {}s remaining in dropout.",
                    sym, remaining
                )
            }
            SafetyState::DrawdownStop => format!(
                "HALTED: Capital drawdown limit exceeded. All stances set to AVOID."
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_normal_to_cautious_transition() {
        let mgr = SafetyManager::new(3, 5, 8, 30.0, 5.0, 80.0);
        mgr.set_initial_capital(dec!(1000)).await;

        mgr.record_trade_outcome("BTC-USDT", true).await;
        mgr.record_trade_outcome("BTC-USDT", true).await;
        let state = mgr.record_trade_outcome("BTC-USDT", true).await;
        assert_eq!(state, SafetyState::Cautious);
        assert_eq!(mgr.consecutive_losses.read().await.get("BTC-USDT"), Some(&3));
    }

    #[tokio::test]
    async fn test_cautious_to_suspended_transition() {
        let mgr = SafetyManager::new(3, 5, 0, 30.0, 5.0, 80.0);
        for _ in 0..5 {
            mgr.record_trade_outcome("BTC-USDT", true).await;
        }
        let state = *mgr.safety_state.read().await;
        assert_eq!(state, SafetyState::Suspended);
        assert_eq!(mgr.consecutive_losses.read().await.get("BTC-USDT"), Some(&5));
    }

    #[tokio::test]
    async fn test_win_resets_per_symbol_counter() {
        let mgr = SafetyManager::new(3, 5, 8, 30.0, 5.0, 80.0);
        for _ in 0..3 {
            mgr.record_trade_outcome("BTC-USDT", true).await;
        }
        assert_eq!(mgr.consecutive_losses.read().await.get("BTC-USDT"), Some(&3));

        mgr.record_trade_outcome("BTC-USDT", false).await;
        assert_eq!(mgr.consecutive_losses.read().await.get("BTC-USDT"), Some(&0));
        let state = *mgr.safety_state.read().await;
        assert_eq!(state, SafetyState::Normal);
    }

    #[tokio::test]
    async fn test_per_symbol_loss_isolation() {
        let mgr = SafetyManager::new(3, 5, 8, 30.0, 5.0, 80.0);

        for _ in 0..4 {
            mgr.record_trade_outcome("BTC-USDT", true).await;
        }
        mgr.record_trade_outcome("ETH-USDT", true).await;
        mgr.record_trade_outcome("ETH-USDT", true).await;

        assert_eq!(mgr.consecutive_losses.read().await.get("BTC-USDT"), Some(&4));
        assert_eq!(mgr.consecutive_losses.read().await.get("ETH-USDT"), Some(&2));
    }

    #[tokio::test]
    async fn test_manual_reset() {
        let mgr = SafetyManager::new(3, 5, 8, 30.0, 5.0, 80.0);
        for _ in 0..4 {
            mgr.record_trade_outcome("BTC-USDT", true).await;
        }
        mgr.reset_consecutive_losses(None).await;
        assert!(mgr.consecutive_losses.read().await.is_empty());
        let state = *mgr.safety_state.read().await;
        assert_eq!(state, SafetyState::Normal);
    }

    #[tokio::test]
    async fn test_drawdown_stop_uses_peak_equity() {
        let mgr = SafetyManager::new(3, 5, 8, 20.0, 5.0, 80.0);
        mgr.set_initial_capital(dec!(1000)).await;
        mgr.set_current_equity(dec!(1200)).await;

        mgr.update_peak_equity(dec!(1200)).await;
        mgr.set_current_equity(dec!(750)).await;

        let result = mgr.check_capital_drawdown().await;
        assert!(result.is_err());
        let state = *mgr.safety_state.read().await;
        assert_eq!(state, SafetyState::DrawdownStop);
    }

    #[tokio::test]
    async fn test_drawdown_not_triggered_within_limit() {
        let mgr = SafetyManager::new(3, 5, 8, 30.0, 5.0, 80.0);
        mgr.set_initial_capital(dec!(1000)).await;
        mgr.set_current_equity(dec!(800)).await;

        mgr.update_peak_equity(dec!(1000)).await;

        let result = mgr.check_capital_drawdown().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_peak_equity_trailing_high_water_mark() {
        let mgr = SafetyManager::new(3, 5, 8, 30.0, 5.0, 80.0);
        mgr.set_initial_capital(dec!(1000)).await;

        assert_eq!(*mgr.peak_equity.read().await, dec!(1000));

        mgr.set_current_equity(dec!(1500)).await;
        assert_eq!(*mgr.peak_equity.read().await, dec!(1500));

        mgr.set_current_equity(dec!(1200)).await;
        assert_eq!(*mgr.peak_equity.read().await, dec!(1500));
    }

    #[tokio::test]
    async fn test_warn_state_triggered_on_daily_drawdown() {
        let mgr = SafetyManager::new(3, 5, 8, 30.0, 5.0, 80.0);
        mgr.set_initial_capital(dec!(10000)).await;
        mgr.set_current_equity(dec!(10000)).await;

        *mgr.starting_session_equity.write().await = dec!(10000);
        *mgr.daily_pnl.write().await = dec!(-600);

        let result = mgr.evaluate_daily_drawdown_warn().await;
        assert_eq!(result, Some(SafetyState::Warn));
        assert_eq!(*mgr.safety_state.read().await, SafetyState::Warn);
    }

    #[tokio::test]
    async fn test_warn_state_clears_when_recovered() {
        let mgr = SafetyManager::new(3, 5, 8, 30.0, 5.0, 80.0);
        mgr.set_initial_capital(dec!(10000)).await;
        mgr.set_current_equity(dec!(10000)).await;

        *mgr.starting_session_equity.write().await = dec!(10000);

        *mgr.daily_pnl.write().await = dec!(-600);
        mgr.evaluate_daily_drawdown_warn().await;
        assert_eq!(*mgr.safety_state.read().await, SafetyState::Warn);

        *mgr.daily_pnl.write().await = dec!(-200);
        mgr.evaluate_daily_drawdown_warn().await;
        assert_eq!(*mgr.safety_state.read().await, SafetyState::Normal);
    }

    #[tokio::test]
    async fn test_trade_blocked_when_suspended() {
        let mgr = SafetyManager::new(3, 3, 1, 30.0, 5.0, 80.0);
        for _ in 0..3 {
            mgr.record_trade_outcome("BTC-USDT", true).await;
        }
        let result = mgr.check_allow_trade("BTC-USDT").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("suspended"));
    }

    #[tokio::test]
    async fn test_other_symbol_unaffected_by_suspension() {
        let mgr = SafetyManager::new(3, 3, 1, 30.0, 5.0, 80.0);
        for _ in 0..3 {
            mgr.record_trade_outcome("BTC-USDT", true).await;
        }
        let result = mgr.check_allow_trade("ETH-USDT").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_evaluate_all_returns_drawdown_first() {
        let mgr = SafetyManager::new(3, 5, 8, 20.0, 5.0, 80.0);
        mgr.set_initial_capital(dec!(1000)).await;
        mgr.set_current_equity(dec!(1200)).await;
        mgr.update_peak_equity(dec!(1200)).await;
        mgr.set_current_equity(dec!(500)).await;

        let triggers = mgr.evaluate_all("BTC-USDT", dec!(0.5), 10.0, false).await;
        assert!(!triggers.is_empty());
        assert_eq!(triggers[0].condition, "drawdown_breach");
        assert_eq!(triggers[0].target_stance, "AVOID");
        assert!(triggers[0].hard_exit);
    }

    #[tokio::test]
    async fn test_session_reset_rebaselines_peak_equity() {
        let mgr = SafetyManager::new(3, 5, 8, 20.0, 5.0, 80.0);
        mgr.set_initial_capital(dec!(1000)).await;
        mgr.set_current_equity(dec!(1500)).await;

        assert_eq!(*mgr.peak_equity.read().await, dec!(1500));

        mgr.session_reset().await;
        assert_eq!(*mgr.peak_equity.read().await, dec!(1500));
        assert_eq!(*mgr.starting_session_equity.read().await, dec!(1500));
        assert_eq!(*mgr.daily_pnl.read().await, dec!(0));
    }
}
