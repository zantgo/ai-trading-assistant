use sqlx::SqlitePool;

pub async fn seed_default_profiles(pool: &SqlitePool) {
    sqlx::query(
        "UPDATE decision_profiles SET profile_name = 'Default' WHERE profile_name = 'Cryptobruj'",
    )
    .execute(pool)
    .await
    .ok();

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM decision_profiles")
        .fetch_one(pool)
        .await
        .unwrap_or((0,));
    if count.0 > 0 {
        return;
    }

    sqlx::query(
        "INSERT INTO decision_profiles (profile_name, long_threshold, short_threshold)
         VALUES ('Default', 40, -40)",
    )
    .execute(pool)
    .await
    .ok();

    let indicators = vec![
        ("RSI (Oversold/Overbought)", 10, "NONE"),
        ("RSI (Divergence)", 20, "NONE"),
        ("MACD (Crossovers)", 10, "NONE"),
        ("MACD (Divergence)", 10, "NONE"),
        ("Support/Resistance", 10, "NONE"),
        ("Trend", 20, "NONE"),
        ("Patterns", 10, "NONE"),
    ];
    for (name, weight, ovr) in &indicators {
        sqlx::query(
            "INSERT INTO profile_indicators (profile_id, indicator_name, weight, override_status)
             VALUES (1, ?1, ?2, ?3)",
        )
        .bind(name)
        .bind(weight)
        .bind(ovr)
        .execute(pool)
        .await
        .ok();
    }

    let risk_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM risk_profiles")
        .fetch_one(pool)
        .await
        .unwrap_or((0,));
    if risk_count.0 == 0 {
        sqlx::query(
            "INSERT INTO risk_profiles (profile_name, capital, max_risk_pct, leverage, commission_pct, funding_rate_8h, spread)
             VALUES ('Risk Profile', 1000.0, 2.0, 20, 0.06, 0.0, 0.0)"
        )
        .execute(pool)
        .await
        .ok();
    }
}

