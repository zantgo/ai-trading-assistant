-- Drops the legacy AI tables (master_assistant_records, agent_thought_logs,
-- decision_memory_buffer, automated_performance_tracker) and their FKs.
-- These tables persisted AI agent decisions, thought-process logs,
-- orchestrator decision memory, and forward-testing accuracy records.
-- With the AI subsystem fully removed, all four tables are obsolete.
-- CASCADE is used because automated_performance_tracker and
-- agent_thought_logs both hold FK references into master_assistant_records
-- and would otherwise fail the drop.

DROP TABLE IF EXISTS agent_thought_logs;
DROP TABLE IF EXISTS automated_performance_tracker;
DROP TABLE IF EXISTS decision_memory_buffer;
DROP TABLE IF EXISTS master_assistant_records;
