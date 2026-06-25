use sqlx::SqlitePool;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DecisionProfile {
    pub id: i64,
    pub profile_name: String,
    pub long_threshold: i32,
    pub short_threshold: i32,
    #[serde(default)]
    pub indicators: Vec<ProfileIndicator>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct ProfileIndicator {
    pub id: i64,
    pub profile_id: i64,
    pub indicator_name: String,
    pub weight: i32,
    pub override_status: String,
}

#[derive(sqlx::FromRow)]
struct DecisionProfileRow {
    id: i64,
    profile_name: String,
    long_threshold: i32,
    short_threshold: i32,
}

pub async fn decision_profiles_list(pool: &SqlitePool) -> Vec<DecisionProfile> {
    let rows: Vec<DecisionProfileRow> = sqlx::query_as(
        "SELECT id, profile_name, long_threshold, short_threshold FROM decision_profiles ORDER BY id ASC",
    )
    .fetch_all(&*pool)
    .await
    .unwrap_or_default();

    let mut profiles = Vec::new();
    for row in rows {
        let indicators = get_profile_indicators_internal(pool, row.id).await;
        profiles.push(DecisionProfile {
            id: row.id,
            profile_name: row.profile_name,
            long_threshold: row.long_threshold,
            short_threshold: row.short_threshold,
            indicators,
        });
    }
    profiles
}

async fn get_profile_indicators_internal(
    pool: &SqlitePool,
    profile_id: i64,
) -> Vec<ProfileIndicator> {
    sqlx::query_as::<_, ProfileIndicator>(
        "SELECT id, profile_id, indicator_name, weight, override_status
         FROM profile_indicators WHERE profile_id = ?1 ORDER BY id ASC",
    )
    .bind(profile_id)
    .fetch_all(&*pool)
    .await
    .unwrap_or_default()
}

pub async fn decision_profile_insert(
    pool: &SqlitePool,
    profile_name: &str,
    long_threshold: i32,
    short_threshold: i32,
) -> i64 {
    match sqlx::query(
        "INSERT INTO decision_profiles (profile_name, long_threshold, short_threshold) VALUES (?1, ?2, ?3)"
    )
    .bind(profile_name).bind(long_threshold).bind(short_threshold)
    .execute(&*pool).await
    {
        Ok(r) => r.last_insert_rowid(),
        Err(e) => { eprintln!("DB: Failed to insert decision profile: {}", e); 0 }
    }
}

pub async fn decision_profile_update(
    pool: &SqlitePool,
    id: i64,
    profile_name: &str,
    long_threshold: i32,
    short_threshold: i32,
) -> bool {
    sqlx::query(
        "UPDATE decision_profiles SET profile_name = ?2, long_threshold = ?3, short_threshold = ?4 WHERE id = ?1"
    )
    .bind(id).bind(profile_name).bind(long_threshold).bind(short_threshold)
    .execute(&*pool).await
    .map(|r| r.rows_affected() > 0)
    .unwrap_or(false)
}

pub async fn decision_profile_delete(pool: &SqlitePool, id: i64) -> bool {
    sqlx::query("DELETE FROM profile_indicators WHERE profile_id = ?1")
        .bind(id)
        .execute(&*pool)
        .await
        .ok();
    sqlx::query("DELETE FROM decision_profiles WHERE id = ?1")
        .bind(id)
        .execute(&*pool)
        .await
        .map(|r| r.rows_affected() > 0)
        .unwrap_or(false)
}

pub async fn profile_indicator_insert(
    pool: &SqlitePool,
    profile_id: i64,
    indicator_name: &str,
    weight: i32,
    override_status: &str,
) -> i64 {
    match sqlx::query(
        "INSERT INTO profile_indicators (profile_id, indicator_name, weight, override_status) VALUES (?1, ?2, ?3, ?4)"
    )
    .bind(profile_id).bind(indicator_name).bind(weight).bind(override_status)
    .execute(&*pool).await
    {
        Ok(r) => r.last_insert_rowid(),
        Err(e) => { eprintln!("DB: Failed to insert profile indicator: {}", e); 0 }
    }
}

pub async fn profile_indicator_update(
    pool: &SqlitePool,
    indicator_id: i64,
    weight: i32,
    override_status: &str,
) -> bool {
    sqlx::query("UPDATE profile_indicators SET weight = ?2, override_status = ?3 WHERE id = ?1")
        .bind(indicator_id)
        .bind(weight)
        .bind(override_status)
        .execute(&*pool)
        .await
        .map(|r| r.rows_affected() > 0)
        .unwrap_or(false)
}

pub async fn profile_indicator_delete(pool: &SqlitePool, indicator_id: i64) -> bool {
    sqlx::query("DELETE FROM profile_indicators WHERE id = ?1")
        .bind(indicator_id)
        .execute(&*pool)
        .await
        .map(|r| r.rows_affected() > 0)
        .unwrap_or(false)
}

// ─── Risk Profiles ────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct RiskProfile {
    pub id: i64,
    pub profile_name: String,
    pub capital: f64,
    pub max_risk_pct: f64,
    pub leverage: i32,
    pub commission_pct: f64,
    pub funding_rate_8h: f64,
    pub spread: f64,
}

pub async fn risk_profiles_list(pool: &SqlitePool) -> Vec<RiskProfile> {
    sqlx::query_as::<_, RiskProfile>(
        "SELECT id, profile_name, capital, max_risk_pct, leverage, commission_pct, funding_rate_8h, spread FROM risk_profiles ORDER BY id ASC"
    )
    .fetch_all(&*pool)
    .await
    .unwrap_or_default()
}

pub async fn risk_profile_by_id(pool: &SqlitePool, id: i64) -> Option<RiskProfile> {
    sqlx::query_as::<_, RiskProfile>(
        "SELECT id, profile_name, capital, max_risk_pct, leverage, commission_pct, funding_rate_8h, spread FROM risk_profiles WHERE id = ?1"
    )
    .bind(id)
    .fetch_optional(&*pool)
    .await
    .ok()
    .flatten()
}

pub async fn risk_profile_insert(
    pool: &SqlitePool,
    profile_name: &str,
    capital: f64,
    max_risk_pct: f64,
    leverage: i32,
    commission_pct: f64,
    funding_rate_8h: f64,
    spread: f64,
) -> i64 {
    match sqlx::query(
        "INSERT INTO risk_profiles (profile_name, capital, max_risk_pct, leverage, commission_pct, funding_rate_8h, spread) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
    )
    .bind(profile_name).bind(capital).bind(max_risk_pct).bind(leverage)
    .bind(commission_pct).bind(funding_rate_8h).bind(spread)
    .execute(&*pool).await
    {
        Ok(r) => r.last_insert_rowid(),
        Err(e) => { eprintln!("DB: Failed to insert risk profile: {}", e); 0 }
    }
}

pub async fn risk_profile_update(
    pool: &SqlitePool,
    id: i64,
    profile_name: &str,
    capital: f64,
    max_risk_pct: f64,
    leverage: i32,
    commission_pct: f64,
    funding_rate_8h: f64,
    spread: f64,
) -> bool {
    sqlx::query(
        "UPDATE risk_profiles SET profile_name = ?2, capital = ?3, max_risk_pct = ?4, leverage = ?5, commission_pct = ?6, funding_rate_8h = ?7, spread = ?8 WHERE id = ?1"
    )
    .bind(id).bind(profile_name).bind(capital).bind(max_risk_pct).bind(leverage)
    .bind(commission_pct).bind(funding_rate_8h).bind(spread)
    .execute(&*pool).await
    .map(|r| r.rows_affected() > 0)
    .unwrap_or(false)
}

pub async fn risk_profile_delete(pool: &SqlitePool, id: i64) -> bool {
    sqlx::query("DELETE FROM risk_profiles WHERE id = ?1")
        .bind(id)
        .execute(&*pool)
        .await
        .map(|r| r.rows_affected() > 0)
        .unwrap_or(false)
}
