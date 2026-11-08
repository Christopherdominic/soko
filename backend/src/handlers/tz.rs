use sqlx::PgPool;

use crate::{
    errors::{ApiError, ApiResult},
    repositories::dashboard::is_known_time_zone,
};

/// Validates an IANA time zone name (e.g. "Africa/Nairobi"); defaults to UTC.
pub async fn resolve(db: &PgPool, tz: Option<&str>) -> ApiResult<String> {
    let tz = tz.map(str::trim).filter(|t| !t.is_empty()).unwrap_or("UTC");
    let well_formed = tz.len() <= 64
        && tz
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "/_+-".contains(c));
    if well_formed && is_known_time_zone(db, tz).await? {
        Ok(tz.to_string())
    } else {
        Err(ApiError::Validation("Unknown time zone".into()))
    }
}
