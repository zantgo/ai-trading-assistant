-- K3 (production audit, 2026-08-17): persist reconstruction provenance.
--
-- Reconstructed candles (REST backfilled ≥1m tiers, EMA/linear-synthesized
-- sub-minute tiers, idle-heartbeat dojis) were never persisted — the
-- `market_snapshots` table carried no provenance column, so a restart or
-- `/api/history` DB fallback lost every gap-filled candle and left
-- permanent holes. Values mirror `ReconstructionMethod` wire tokens
-- ("SYNTHETIC" | "EXCHANGE_HISTORICAL"); NULL = genuine live candle.

ALTER TABLE market_snapshots ADD COLUMN reconstructed TEXT;
