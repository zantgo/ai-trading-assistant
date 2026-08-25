-- v10.2 institutional: R-multiple and symbol attribution for backtest trades
ALTER TABLE backtest_trades ADD COLUMN r_multiple REAL;
ALTER TABLE backtest_trades ADD COLUMN symbol TEXT;
