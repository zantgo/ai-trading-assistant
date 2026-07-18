pub mod engine;
pub mod gates;
pub mod order;
pub mod state_machine;

pub use engine::ExecutionEngine;
pub use gates::{GateResult, evaluate_gates};
pub use order::construct_order;
pub use state_machine::{OrderLifecycle, OrderTransition};
