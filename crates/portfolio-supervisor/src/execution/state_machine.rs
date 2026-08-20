use config_models::{OrderPacket, OrderStatus};

#[derive(Debug, Clone)]
pub struct OrderTransition {
    pub from: OrderStatus,
    pub to: OrderStatus,
    pub timestamp_ms: u64,
    pub metadata: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OrderLifecycle {
    pub packet: OrderPacket,
    pub exchange_order_id: Option<String>,
    pub status: OrderStatus,
    pub filled_size: rust_decimal::Decimal,
    pub fill_price: Option<rust_decimal::Decimal>,
    pub slippage_bps: Option<f64>,
    pub transitions: Vec<OrderTransition>,
    pub created_at: u64,
}

impl OrderLifecycle {
    pub fn new(packet: OrderPacket, created_at: u64) -> Self {
        Self {
            packet,
            exchange_order_id: None,
            status: OrderStatus::Pending,
            filled_size: rust_decimal::Decimal::ZERO,
            fill_price: None,
            slippage_bps: None,
            transitions: vec![],
            created_at,
        }
    }

    pub fn transition(
        &mut self,
        to: OrderStatus,
        timestamp_ms: u64,
        metadata: Option<String>,
    ) -> Result<(), String> {
        let valid = matches!(
            (self.status, to),
            (OrderStatus::Pending, OrderStatus::Submitted)
                | (OrderStatus::Pending, OrderStatus::Rejected)
                | (OrderStatus::Pending, OrderStatus::Cancelled)
                | (OrderStatus::Submitted, OrderStatus::Open)
                | (OrderStatus::Submitted, OrderStatus::Rejected)
                | (OrderStatus::PreDispatch, OrderStatus::Pending)
                | (OrderStatus::PreDispatch, OrderStatus::Rejected)
                | (OrderStatus::Open, OrderStatus::PartiallyFilled)
                | (OrderStatus::Open, OrderStatus::Cancelled)
                | (OrderStatus::PartiallyFilled, OrderStatus::PartiallyFilled)
                | (OrderStatus::PartiallyFilled, OrderStatus::Closed)
                | (OrderStatus::PartiallyFilled, OrderStatus::Cancelled)
                | (OrderStatus::Open, OrderStatus::Closed)
        );

        if !valid {
            return Err(format!("Invalid transition: {:?} -> {:?}", self.status, to));
        }

        self.transitions.push(OrderTransition {
            from: self.status,
            to,
            timestamp_ms,
            metadata,
        });
        self.status = to;
        Ok(())
    }

    pub fn record_fill(
        &mut self,
        fill_qty: rust_decimal::Decimal,
        fill_price: rust_decimal::Decimal,
        timestamp_ms: u64,
    ) {
        self.filled_size += fill_qty;
        self.fill_price = Some(fill_price);
        let _ = self.transition(
            OrderStatus::PartiallyFilled,
            timestamp_ms,
            Some(format!("filled {} @ {}", fill_qty, fill_price)),
        );
    }
}
