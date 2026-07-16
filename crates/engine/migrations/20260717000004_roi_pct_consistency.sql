-- Migration: ROI field consistency — `roi_percentage` → `roi_pct` (DB-20)
-- The canonical ROI key across paper_trades and trade_telemetry_history is `roi_pct`.
-- The trade_learning_journal retains `roi_percentage` (legacy) until a future migration
-- aligns it.

ALTER TABLE trade_telemetry_history RENAME COLUMN roi_percentage TO roi_pct;
ALTER TABLE trade_learning_journal RENAME COLUMN roe_percentage TO roi_pct;
