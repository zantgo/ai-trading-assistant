use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalPosition {
    pub symbol: String,
    pub direction: String,
    pub size: f64,
    pub entry_price: f64,
    pub liquidation_price: Option<f64>,
    pub unrealized_pnl: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangePosition {
    pub symbol: String,
    pub position_value: f64,
    pub entry_price: f64,
    pub liquidation_price: Option<f64>,
    pub unrealized_pnl: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Discrepancy {
    pub symbol: String,
    pub field: String,
    pub local_value: String,
    pub exchange_value: String,
    pub severity: DiscrepancySeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DiscrepancySeverity {
    Info,
    Warning,
    Critical,
}

pub struct PositionReconciler {
    pub last_reconciled: i64,
    pub max_discrepancy_pct: f64,
}

impl PositionReconciler {
    pub fn new(max_discrepancy_pct: f64) -> Self {
        Self {
            last_reconciled: 0,
            max_discrepancy_pct,
        }
    }

    pub fn reconcile(&self, local: &[LocalPosition], exchange: &[ExchangePosition]) -> Vec<Discrepancy> {
        let mut discrepancies = Vec::new();

        for lp in local {
            let ep = exchange.iter().find(|e| e.symbol == lp.symbol);
            match ep {
                Some(ep) => {
                    let size_diff = (lp.size - ep.position_value.abs()).abs();
                    if size_diff > 0.0 {
                        let pct = if lp.size > 0.0 { size_diff / lp.size } else { 1.0 };
                        let severity = if pct > self.max_discrepancy_pct { DiscrepancySeverity::Critical }
                            else if pct > self.max_discrepancy_pct * 0.5 { DiscrepancySeverity::Warning }
                            else { DiscrepancySeverity::Info };
                        discrepancies.push(Discrepancy {
                            symbol: lp.symbol.clone(),
                            field: "size".into(),
                            local_value: format!("{:.6}", lp.size),
                            exchange_value: format!("{:.6}", ep.position_value.abs()),
                            severity,
                        });
                    }
                    let entry_diff = (lp.entry_price - ep.entry_price).abs();
                    if entry_diff / lp.entry_price.max(1.0) > 0.01 {
                        discrepancies.push(Discrepancy {
                            symbol: lp.symbol.clone(),
                            field: "entry_price".into(),
                            local_value: format!("{:.2}", lp.entry_price),
                            exchange_value: format!("{:.2}", ep.entry_price),
                            severity: DiscrepancySeverity::Warning,
                        });
                    }
                }
                None => {
                    discrepancies.push(Discrepancy {
                        symbol: lp.symbol.clone(),
                        field: "missing".into(),
                        local_value: format!("size={}", lp.size),
                        exchange_value: "NONE".into(),
                        severity: DiscrepancySeverity::Critical,
                    });
                }
            }
        }

        discrepancies
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_discrepancy() {
        let r = PositionReconciler::new(0.05);
        let local = vec![LocalPosition {
            symbol: "BTC".into(), direction: "long".into(), size: 0.1,
            entry_price: 50000.0, liquidation_price: None, unrealized_pnl: 100.0,
        }];
        let exchange = vec![ExchangePosition {
            symbol: "BTC".into(), position_value: 0.1,
            entry_price: 50000.0, liquidation_price: None, unrealized_pnl: 100.0,
        }];
        let d = r.reconcile(&local, &exchange);
        assert!(d.is_empty());
    }

    #[test]
    fn test_size_discrepancy() {
        let r = PositionReconciler::new(0.05);
        let local = vec![LocalPosition {
            symbol: "ETH".into(), direction: "short".into(), size: 2.0,
            entry_price: 3000.0, liquidation_price: None, unrealized_pnl: -50.0,
        }];
        let exchange = vec![ExchangePosition {
            symbol: "ETH".into(), position_value: 1.5,
            entry_price: 3000.0, liquidation_price: None, unrealized_pnl: -50.0,
        }];
        let d = r.reconcile(&local, &exchange);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].field, "size");
    }

    #[test]
    fn test_missing_position() {
        let r = PositionReconciler::new(0.05);
        let local = vec![LocalPosition {
            symbol: "SOL".into(), direction: "long".into(), size: 10.0,
            entry_price: 100.0, liquidation_price: None, unrealized_pnl: 10.0,
        }];
        let exchange = vec![];
        let d = r.reconcile(&local, &exchange);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].severity, DiscrepancySeverity::Critical);
    }
}
