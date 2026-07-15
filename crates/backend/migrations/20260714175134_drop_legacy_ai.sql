-- Drops the legacy AI tables (master_assistant_records, agent_thought_logs,
-- decision_memory_buffer, automated_performance_tracker, individual_indicator_logs)
-- and their FKs.
-- These tables persisted AI agent decisions, thought-process logs,
-- orchestrator decision memory, forward-testing accuracy records, and
-- per-indicator signal telemetry keyed to master record IDs.
-- With the AI subsystem fully removed, all five tables are obsolete.

DROP TABLE IF EXISTS agent_thought_logs;
DROP TABLE IF EXISTS automated_performance_tracker;
DROP TABLE IF EXISTS decision_memory_buffer;
DROP TABLE IF EXISTS individual_indicator_logs;
DROP TABLE IF EXISTS master_assistant_records;
