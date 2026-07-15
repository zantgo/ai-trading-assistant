-- Drops the paper_balances and paper_trades tables used by the legacy
-- manual paper-trading matching engine. With manual trading removed,
-- neither table has writers. Dashboard and stats continue to read
-- automated trade telemetry from trade_telemetry_history.

DROP TABLE IF EXISTS paper_trades;
DROP TABLE IF EXISTS paper_balances;
