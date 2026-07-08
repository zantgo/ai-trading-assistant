-- Add market_regime column to paper_trades for regime-specific performance analysis.
ALTER TABLE paper_trades ADD COLUMN market_regime TEXT;
