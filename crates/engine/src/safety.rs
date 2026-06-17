use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Debug, Clone, PartialEq)]
pub enum CautionLevel {
    Normal,
    Cautious,
    Suspended,
    DrawdownStop,
}

impl CautionLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            CautionLevel::Normal => "normal",
            CautionLevel::Cautious => "cautious",
            CautionLevel::Suspended => "suspended",
            CautionLevel::DrawdownStop => "drawdown_stop",
        }
    }
}

pub struct SafetyManager {
    pub consecutive_losses: AtomicU32,
    pub caution_level: RwLock<CautionLevel>,
    pub dropout_until: RwLock<Option<Instant>>,
    pub initial_capital: RwLock<f64>,
    pub current_equity: RwLock<f64>,

    // Configurable thresholds
    pub caution_threshold: u32,
    pub dropout_threshold: u32,
    pub dropout_duration: Duration,
    pub drawdown_pct: f64,
}

impl SafetyManager {
    pub fn new(
        caution_threshold: u32,
        dropout_threshold: u32,
        dropout_duration_hours: u64,
        drawdown_pct: f64,
    ) -> Self {
        Self {
            consecutive_losses: AtomicU32::new(0),
            caution_level: RwLock::new(CautionLevel::Normal),
            dropout_until: RwLock::new(None),
            initial_capital: RwLock::new(0.0),
            current_equity: RwLock::new(0.0),
            caution_threshold,
            dropout_threshold,
            dropout_duration: Duration::from_secs(dropout_duration_hours * 3600),
            drawdown_pct,
        }
    }

    /// Record a trade outcome. Returns the new caution level.
    pub async fn record_trade_outcome(&self, is_loss: bool) -> CautionLevel {
        if is_loss {
            let count = self.consecutive_losses.fetch_add(1, Ordering::Relaxed) + 1;

            if count >= self.dropout_threshold {
                let until = Instant::now() + self.dropout_duration;
                *self.dropout_until.write().await = Some(until);
                *self.caution_level.write().await = CautionLevel::Suspended;
                eprintln!(
                    "🛑 SAFETY: Instance suspended for {}h after {} consecutive losses",
                    self.dropout_duration.as_secs() / 3600,
                    count
                );
            } else if count >= self.caution_threshold {
                *self.caution_level.write().await = CautionLevel::Cautious;
                eprintln!(
                    "⚠️  SAFETY: Instance entering Cautious mode after {} consecutive losses",
                    count
                );
            }
        } else {
            // Win resets the counter
            self.consecutive_losses.store(0, Ordering::Relaxed);
            *self.caution_level.write().await = CautionLevel::Normal;
            *self.dropout_until.write().await = None;
        }

        self.caution_level.read().await.clone()
    }

    /// Check if trading is currently allowed. Returns Ok if allowed, Err with reason if blocked.
    pub async fn check_allow_trade(&self) -> Result<(), String> {
        let level = self.caution_level.read().await.clone();
        match level {
            CautionLevel::DrawdownStop => {
                return Err("Trading halted: capital drawdown limit exceeded".into());
            }
            CautionLevel::Suspended => {
                // Check if dropout period has elapsed
                if let Some(until) = *self.dropout_until.read().await {
                    if Instant::now() < until {
                        let remaining = until.duration_since(Instant::now()).as_secs();
                        return Err(format!(
                            "Trading suspended: dropout period active for {} more seconds",
                            remaining
                        ));
                    }
                }
                // Dropout period elapsed, clear suspension
                *self.caution_level.write().await = CautionLevel::Normal;
                *self.dropout_until.write().await = None;
            }
            _ => {}
        }
        Ok(())
    }

    /// Check capital drawdown and potentially trigger drawdown stop.
    pub async fn check_capital_drawdown(&self) -> Result<(), String> {
        let initial = *self.initial_capital.read().await;
        let current = *self.current_equity.read().await;

        if initial <= 0.0 {
            return Ok(());
        }

        let loss_pct = ((initial - current) / initial) * 100.0;
        if loss_pct >= self.drawdown_pct {
            *self.caution_level.write().await = CautionLevel::DrawdownStop;
            eprintln!(
                "🛑 SAFETY: Drawdown stop triggered. Loss {:.1}% exceeds {:.1}% limit",
                loss_pct, self.drawdown_pct
            );
            return Err(format!(
                "Capital drawdown {:.1}% exceeds {:.1}% limit",
                loss_pct, self.drawdown_pct
            ));
        }
        Ok(())
    }

    /// Reset the consecutive loss counter (manual override).
    pub async fn reset_consecutive_losses(&self) {
        self.consecutive_losses.store(0, Ordering::Relaxed);
        *self.caution_level.write().await = CautionLevel::Normal;
        *self.dropout_until.write().await = None;
        println!("🔄 SAFETY: Consecutive loss counter manually reset");
    }

    /// Set initial capital (called at instance creation or manual capital update).
    pub async fn set_initial_capital(&self, capital: f64) {
        *self.initial_capital.write().await = capital;
    }

    /// Update current equity (called after each trade or valuation).
    pub async fn set_current_equity(&self, equity: f64) {
        *self.current_equity.write().await = equity;
    }

    /// Get the current caution level for AI context.
    pub async fn get_caution_context(&self) -> String {
        let level = self.caution_level.read().await.clone();
        let losses = self.consecutive_losses.load(Ordering::Relaxed);
        match level {
            CautionLevel::Normal => format!("Normal risk mode. {} consecutive losses.", losses),
            CautionLevel::Cautious => format!(
                "CAUTION: {} consecutive losses. Tighten entry criteria, reduce position sizing.",
                losses
            ),
            CautionLevel::Suspended => {
                let remaining = self.dropout_until.read().await
                    .map(|u| u.duration_since(Instant::now()).as_secs())
                    .unwrap_or(0);
                format!("SUSPENDED: {} consecutive losses. {}s remaining.", losses, remaining)
            }
            CautionLevel::DrawdownStop => format!(
                "HALTED: Capital drawdown limit exceeded. {} consecutive losses.",
                losses
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_normal_to_cautious_transition() {
        let mgr = SafetyManager::new(3, 5, 8, 30.0);
        mgr.set_initial_capital(1000.0).await;

        mgr.record_trade_outcome(true).await;
        mgr.record_trade_outcome(true).await;
        let level = mgr.record_trade_outcome(true).await;
        assert_eq!(level, CautionLevel::Cautious);
        assert_eq!(mgr.consecutive_losses.load(Ordering::Relaxed), 3);
    }

    #[tokio::test]
    async fn test_cautious_to_suspended_transition() {
        let mgr = SafetyManager::new(3, 5, 0, 30.0);
        for _ in 0..5 {
            mgr.record_trade_outcome(true).await;
        }
        let level = mgr.caution_level.read().await.clone();
        assert_eq!(level, CautionLevel::Suspended);
        assert_eq!(mgr.consecutive_losses.load(Ordering::Relaxed), 5);
    }

    #[tokio::test]
    async fn test_win_resets_counter() {
        let mgr = SafetyManager::new(3, 5, 8, 30.0);
        for _ in 0..3 {
            mgr.record_trade_outcome(true).await;
        }
        assert_eq!(mgr.consecutive_losses.load(Ordering::Relaxed), 3);

        mgr.record_trade_outcome(false).await; // Win
        assert_eq!(mgr.consecutive_losses.load(Ordering::Relaxed), 0);
        let level = mgr.caution_level.read().await.clone();
        assert_eq!(level, CautionLevel::Normal);
    }

    #[tokio::test]
    async fn test_manual_reset() {
        let mgr = SafetyManager::new(3, 5, 8, 30.0);
        for _ in 0..4 {
            mgr.record_trade_outcome(true).await;
        }
        mgr.reset_consecutive_losses().await;
        assert_eq!(mgr.consecutive_losses.load(Ordering::Relaxed), 0);
        let level = mgr.caution_level.read().await.clone();
        assert_eq!(level, CautionLevel::Normal);
    }

    #[tokio::test]
    async fn test_drawdown_stop() {
        let mgr = SafetyManager::new(3, 5, 8, 20.0);
        mgr.set_initial_capital(1000.0).await;
        mgr.set_current_equity(750.0).await; // 25% loss
        let result = mgr.check_capital_drawdown().await;
        assert!(result.is_err());
        let level = mgr.caution_level.read().await.clone();
        assert_eq!(level, CautionLevel::DrawdownStop);
    }

    #[tokio::test]
    async fn test_drawdown_not_triggered_within_limit() {
        let mgr = SafetyManager::new(3, 5, 8, 30.0);
        mgr.set_initial_capital(1000.0).await;
        mgr.set_current_equity(800.0).await; // 20% loss, below 30% limit
        let result = mgr.check_capital_drawdown().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_caution_context() {
        let mgr = SafetyManager::new(3, 5, 8, 30.0);
        for _ in 0..3 {
            mgr.record_trade_outcome(true).await;
        }
        let ctx = mgr.get_caution_context().await;
        assert!(ctx.contains("CAUTION"));
        assert!(ctx.contains("3 consecutive losses"));
    }

    #[tokio::test]
    async fn test_trade_blocked_when_suspended() {
        let mgr = SafetyManager::new(3, 3, 1, 30.0); // 3 = dropout threshold with instant trigger
        for _ in 0..3 {
            mgr.record_trade_outcome(true).await;
        }
        let result = mgr.check_allow_trade().await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("suspended"));
    }
}
