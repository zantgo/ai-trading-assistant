pub mod engine;
pub mod gates;
pub mod order;
pub mod state_machine;

pub use engine::{CapitalState, ExecutionEngine, PositionRecord};
pub use gates::{evaluate_gates, GateResult};
pub use order::construct_order;
pub use state_machine::{OrderLifecycle, OrderTransition};
