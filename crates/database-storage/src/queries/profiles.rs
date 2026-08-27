use rust_decimal::Decimal;
use sqlx::SqlitePool;
use std::str::FromStr;

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
    .fetch_all(pool)
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
    .fetch_all(pool)
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
    .execute(pool).await
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
    .execute(pool).await
    .map(|r| r.rows_affected() > 0)
    .unwrap_or(false)
}

pub async fn decision_profile_delete(pool: &SqlitePool, id: i64) -> bool {
    sqlx::query("DELETE FROM profile_indicators WHERE profile_id = ?1")
        .bind(id)
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM decision_profiles WHERE id = ?1")
        .bind(id)
        .execute(pool)
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
    .execute(pool).await
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
        .execute(pool)
        .await
        .map(|r| r.rows_affected() > 0)
        .unwrap_or(false)
}

pub async fn profile_indicator_delete(pool: &SqlitePool, indicator_id: i64) -> bool {
    sqlx::query("DELETE FROM profile_indicators WHERE id = ?1")
        .bind(indicator_id)
        .execute(pool)
        .await
        .map(|r| r.rows_affected() > 0)
        .unwrap_or(false)
}

// ─── Risk Profiles ────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RiskProfile {
    pub id: i64,
    pub profile_name: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub capital: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub max_risk_pct: Decimal,
    pub leverage: i32,
    #[serde(with = "rust_decimal::serde::str")]
    pub commission_pct: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub funding_rate_8h: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub spread: Decimal,
}

fn parse_decimal_field(s: &str, field: &str) -> Result<Decimal, sqlx::Error> {
    Decimal::from_str(s).map_err(|e| sqlx::Error::ColumnDecode {
        index: field.to_string(),
        source: Box::new(e),
    })
}

impl<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow> for RiskProfile {
    fn from_row(row: &'r sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        let id: i64 = row.try_get("id")?;
        let profile_name: String = row.try_get("profile_name")?;
        let capital_str: String = row.try_get("capital")?;
        let max_risk_str: String = row.try_get("max_risk_pct")?;
        let leverage: i32 = row.try_get("leverage")?;
        let commission_str: String = row.try_get("commission_pct")?;
        let funding_str: String = row.try_get("funding_rate_8h")?;
        let spread_str: String = row.try_get("spread")?;

        Ok(RiskProfile {
            id,
            profile_name,
            capital: parse_decimal_field(&capital_str, "capital")?,
            max_risk_pct: parse_decimal_field(&max_risk_str, "max_risk_pct")?,
            leverage,
            commission_pct: parse_decimal_field(&commission_str, "commission_pct")?,
            funding_rate_8h: parse_decimal_field(&funding_str, "funding_rate_8h")?,
            spread: parse_decimal_field(&spread_str, "spread")?,
        })
    }
}

pub async fn risk_profiles_list(pool: &SqlitePool) -> Vec<RiskProfile> {
    sqlx::query_as::<_, RiskProfile>(
        "SELECT id, profile_name, capital, max_risk_pct, leverage, commission_pct, funding_rate_8h, spread FROM risk_profiles ORDER BY id ASC"
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}

pub async fn risk_profile_by_id(pool: &SqlitePool, id: i64) -> Option<RiskProfile> {
    sqlx::query_as::<_, RiskProfile>(
        "SELECT id, profile_name, capital, max_risk_pct, leverage, commission_pct, funding_rate_8h, spread FROM risk_profiles WHERE id = ?1"
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
}

#[allow(clippy::too_many_arguments)]
pub async fn risk_profile_insert(
    pool: &SqlitePool,
    profile_name: &str,
    capital: Decimal,
    max_risk_pct: Decimal,
    leverage: i32,
    commission_pct: Decimal,
    funding_rate_8h: Decimal,
    spread: Decimal,
) -> i64 {
    match sqlx::query(
        "INSERT INTO risk_profiles (profile_name, capital, max_risk_pct, leverage, commission_pct, funding_rate_8h, spread) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
    )
    .bind(profile_name).bind(capital.to_string()).bind(max_risk_pct.to_string()).bind(leverage)
    .bind(commission_pct.to_string()).bind(funding_rate_8h.to_string()).bind(spread.to_string())
    .execute(pool).await
    {
        Ok(r) => r.last_insert_rowid(),
        Err(e) => { eprintln!("DB: Failed to insert risk profile: {}", e); 0 }
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn risk_profile_update(
    pool: &SqlitePool,
    id: i64,
    profile_name: &str,
    capital: Decimal,
    max_risk_pct: Decimal,
    leverage: i32,
    commission_pct: Decimal,
    funding_rate_8h: Decimal,
    spread: Decimal,
) -> bool {
    sqlx::query(
        "UPDATE risk_profiles SET profile_name = ?2, capital = ?3, max_risk_pct = ?4, leverage = ?5, commission_pct = ?6, funding_rate_8h = ?7, spread = ?8 WHERE id = ?1"
    )
    .bind(id).bind(profile_name).bind(capital.to_string()).bind(max_risk_pct.to_string()).bind(leverage)
    .bind(commission_pct.to_string()).bind(funding_rate_8h.to_string()).bind(spread.to_string())
    .execute(pool).await
    .map(|r| r.rows_affected() > 0)
    .unwrap_or(false)
}

pub async fn risk_profile_delete(pool: &SqlitePool, id: i64) -> bool {
    sqlx::query("DELETE FROM risk_profiles WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await
        .map(|r| r.rows_affected() > 0)
        .unwrap_or(false)
}
