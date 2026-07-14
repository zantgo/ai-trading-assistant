// ─── JSON-RPC 2.0 Method Names (Standardized) ────────────────────────

// Risk / Execution layer
pub const METHOD_EXECUTION_OPEN: &str = "execution.open_position";
pub const METHOD_EXECUTION_CLOSE: &str = "execution.close_position";
pub const METHOD_EXECUTION_ADJUST: &str = "execution.adjust_position";
pub const METHOD_EXECUTION_VALIDATE: &str = "execution.validate";

// Safety subsystem
pub const METHOD_SAFETY_CHECK: &str = "safety.check";
pub const METHOD_SAFETY_DROPOUT_TRIGGERED: &str = "safety.dropout_triggered";
pub const METHOD_SAFETY_RESET: &str = "safety.reset";

// Instance management
pub const METHOD_INSTANCE_CREATE: &str = "instance.create";
pub const METHOD_INSTANCE_PAUSE: &str = "instance.pause";
pub const METHOD_INSTANCE_STOP: &str = "instance.stop";
pub const METHOD_INSTANCE_DELETE: &str = "instance.delete";
pub const METHOD_INSTANCE_STATUS_CHANGED: &str = "instance.status_changed";

// Broadcast to frontend
pub const METHOD_BROADCAST_MARKET_SNAPSHOT: &str = "broadcast.market_snapshot";
pub const METHOD_BROADCAST_SAFETY_EVENT: &str = "broadcast.safety_event";
pub const METHOD_BROADCAST_SYSTEM_STATUS: &str = "broadcast.system_status";

// Configuration
pub const METHOD_CONFIG_UPDATE: &str = "config.update";
pub const METHOD_CONFIG_QUERY: &str = "config.query";

// Session
pub const METHOD_SESSION_INIT: &str = "session.init";
pub const METHOD_SESSION_QUIT: &str = "session.quit";
pub const METHOD_SESSION_STATUS: &str = "session.status";
