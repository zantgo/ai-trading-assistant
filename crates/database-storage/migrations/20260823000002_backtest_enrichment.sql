-- v10: backtest trade enrichment — entry timestamp, hold time, MFE/MAE, ROI.

ALTER TABLE backtest_trades ADD COLUMN ts_entry_secs INTEGER;
ALTER TABLE backtest_trades ADD COLUMN hold_secs INTEGER;
ALTER TABLE backtest_trades ADD COLUMN mfe_pct REAL;
ALTER TABLE backtest_trades ADD COLUMN mae_pct REAL;
ALTER TABLE backtest_trades ADD COLUMN roi_pct REAL;
