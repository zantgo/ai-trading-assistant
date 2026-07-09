-- Remove AI/LLM and Bitget artifacts from the database
-- Part of the AI → Deterministic refactoring

-- Drop AI agent tables
DROP TABLE IF EXISTS trade_learning_journal;
DROP TABLE IF EXISTS agent_thought_logs;
DROP TABLE IF EXISTS master_assistant_records;

-- Remove Bitget exchange keys
DELETE FROM exchange_keys WHERE LOWER(exchange) = 'bitget';
