-- v10.1: backtest trade cost attribution — per-trade slippage, commission
-- fees, and funding fees (direction-aware settlement accrual).

ALTER TABLE backtest_trades ADD COLUMN slippage_bps REAL;
ALTER TABLE backtest_trades ADD COLUMN commission_fees REAL;
ALTER TABLE backtest_trades ADD COLUMN funding_fees REAL;
