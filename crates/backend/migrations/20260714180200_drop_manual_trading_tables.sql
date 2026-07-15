-- Drops manual-trading tables: active positions, open orders, slot tracking,
-- take-profit targets, position equity snapshots, and the trade learning journal.
-- With manual trading removed, the platform only writes automated trade telemetry
-- to trade_telemetry_history. All user_trades, risk_profiles, decision_profiles,
-- and monitoring tables are unaffected.

DROP TABLE IF EXISTS position_equity_snapshots;
DROP TABLE IF EXISTS position_slots;
DROP TABLE IF EXISTS open_orders;
DROP TABLE IF EXISTS position_take_profit_targets;
DROP TABLE IF EXISTS active_position_portions;
DROP TABLE IF EXISTS trade_learning_journal;
DROP TABLE IF EXISTS active_positions;
