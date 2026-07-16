-- Migrate risk_profiles columns from REAL to TEXT for full Decimal precision.
-- SQLite has no native DECIMAL type; TEXT preserves the canonical string form
-- written by rust_decimal::Decimal::to_string() and round-trips exactly.
--
-- The CAST(REAL AS TEXT) emits the shortest decimal representation that
-- round-trips back to the same DOUBLE — clean values like "0.06", not the
-- 17-digit artifact "0.060000000000000005".

CREATE TABLE IF NOT EXISTS risk_profiles_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    profile_name TEXT NOT NULL UNIQUE,
    capital TEXT NOT NULL DEFAULT '1000',
    max_risk_pct TEXT NOT NULL DEFAULT '2',
    leverage INTEGER NOT NULL DEFAULT 20,
    commission_pct TEXT NOT NULL DEFAULT '0.06',
    funding_rate_8h TEXT NOT NULL DEFAULT '0',
    spread TEXT NOT NULL DEFAULT '0'
);

INSERT INTO risk_profiles_new (id, profile_name, capital, max_risk_pct, leverage,
                               commission_pct, funding_rate_8h, spread)
SELECT id, profile_name,
       CAST(capital AS TEXT),
       CAST(max_risk_pct AS TEXT),
       leverage,
       CAST(commission_pct AS TEXT),
       CAST(funding_rate_8h AS TEXT),
       CAST(spread AS TEXT)
FROM risk_profiles;

DROP TABLE risk_profiles;
ALTER TABLE risk_profiles_new RENAME TO risk_profiles;