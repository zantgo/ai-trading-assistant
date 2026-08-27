-- v7.0: rename strategy_analytics_history.policy_id -> setup_type
-- (the v7 grouping key is the setup type; "policy" is erased).
ALTER TABLE strategy_analytics_history RENAME COLUMN policy_id TO setup_type;
