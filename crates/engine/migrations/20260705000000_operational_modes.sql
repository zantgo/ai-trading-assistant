-- Migration: Operational Modes, Trigger Tracking & Leverage Scaling
-- Adds columns to master_assistant_records and paper_balances for the
-- operational-mode architecture.

-- Telemetry Records: track which mode and trigger produced each AI run
ALTER TABLE master_assistant_records ADD COLUMN operational_mode TEXT NOT NULL DEFAULT 'HybridAiCopilot';
ALTER TABLE master_assistant_records ADD COLUMN trigger_type_detail TEXT;
ALTER TABLE master_assistant_records ADD COLUMN indicator_weights_json TEXT;

-- Paper Balances: support dynamic leverage scaling per pair
ALTER TABLE paper_balances ADD COLUMN leverage_mode TEXT NOT NULL DEFAULT 'Fixed';
ALTER TABLE paper_balances ADD COLUMN leverage_cap INTEGER NOT NULL DEFAULT 20;
ALTER TABLE paper_balances ADD COLUMN atr_leverage_multiplier REAL NOT NULL DEFAULT 0.0;
