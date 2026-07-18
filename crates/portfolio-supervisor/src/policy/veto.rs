use config_models::Stance;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct VetoEvent {
    pub symbol: String,
    pub target_stance: Stance,
    pub reason: String,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VetoPhase {
    HardExit,
    DiscardPending,
    CommitStance,
    CancelRemaining,
    NullifyEntry,
    Audit,
}

pub struct VetoHandler {
    event: Option<VetoEvent>,
    phase: VetoPhase,
    pub hard_exit_ack_timeout_ms: u64,
}

impl VetoHandler {
    pub fn new(hard_exit_ack_timeout_ms: u64) -> Self {
        Self {
            event: None,
            phase: VetoPhase::HardExit,
            hard_exit_ack_timeout_ms,
        }
    }

    pub fn initiate(&mut self, event: VetoEvent) {
        self.event = Some(event);
        self.phase = VetoPhase::HardExit;
    }

    pub fn is_active(&self) -> bool {
        self.event.is_some()
    }

    pub fn target_stance(&self) -> Option<Stance> {
        self.event.as_ref().map(|e| e.target_stance)
    }

    pub fn symbol(&self) -> Option<&str> {
        self.event.as_ref().map(|e| e.symbol.as_str())
    }

    pub fn current_phase(&self) -> VetoPhase {
        self.phase
    }

    pub fn advance_phase(&mut self) {
        self.phase = match self.phase {
            VetoPhase::HardExit => VetoPhase::DiscardPending,
            VetoPhase::DiscardPending => VetoPhase::CommitStance,
            VetoPhase::CommitStance => VetoPhase::CancelRemaining,
            VetoPhase::CancelRemaining => VetoPhase::NullifyEntry,
            VetoPhase::NullifyEntry => VetoPhase::Audit,
            VetoPhase::Audit => VetoPhase::Audit,
        };
    }

    pub fn is_complete(&self) -> bool {
        self.event.is_some() && self.phase == VetoPhase::Audit
    }

    pub fn consume_event(&mut self) -> Option<VetoEvent> {
        self.phase = VetoPhase::HardExit;
        self.event.take()
    }

    pub fn needs_hard_exit(&self) -> bool {
        if let Some(ref event) = self.event {
            event.target_stance == Stance::Avoid && self.phase == VetoPhase::HardExit
        } else {
            false
        }
    }
}

#[derive(Debug, Clone)]
pub struct VetoLog {
    pub timestamp_ms: u64,
    pub symbol: String,
    pub from_stance: Stance,
    pub to_stance: Stance,
    pub reason: String,
    pub hard_exit_dispatched: bool,
}

impl VetoLog {
    pub fn new(event: &VetoEvent, from_stance: Stance, hard_exit_dispatched: bool) -> Self {
        Self {
            timestamp_ms: event.timestamp_ms,
            symbol: event.symbol.clone(),
            from_stance,
            to_stance: event.target_stance,
            reason: event.reason.clone(),
            hard_exit_dispatched,
        }
    }
}
