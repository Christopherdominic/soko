use axum::{
    Json,
    extract::{Query, State},
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use super::tz;
use crate::{
    errors::ApiResult, middleware::auth::CurrentMerchant, repositories::dashboard, state::AppState,
};

#[derive(Deserialize)]
pub struct SummaryQuery {
    tz: Option<String>,
}

#[derive(Serialize)]
pub struct DaySales {
    day: NaiveDate,
    sales_sats: i64,
    count: i64,
}

#[derive(Serialize)]
pub struct SummaryResponse {
    time_zone: String,
    total_sales_sats: i64,
    paid_count: i64,
    pending_count: i64,
    today_sales_sats: i64,
    today_count: i64,
    last_7_days: Vec<DaySales>,
}

/// GET /api/dashboard/summary?tz=Africa/Nairobi
/// Only real payments count; demo rows are excluded.
pub async fn summary(
    State(state): State<AppState>,
    CurrentMerchant(merchant): CurrentMerchant,
    Query(q): Query<SummaryQuery>,
) -> ApiResult<Json<SummaryResponse>> {
    let tz = tz::resolve(&state.db, q.tz.as_deref()).await?;
    let s = dashboard::summary(&state.db, merchant.id, &tz).await?;
    let days = dashboard::daily_sales(&state.db, merchant.id, &tz, 7).await?;
    Ok(Json(SummaryResponse {
        time_zone: tz,
        total_sales_sats: s.total_sales_sats,
        paid_count: s.paid_count,
        pending_count: s.pending_count,
        today_sales_sats: s.today_sales_sats,
        today_count: s.today_count,
        last_7_days: days
            .into_iter()
            .map(|d| DaySales {
                day: d.day,
                sales_sats: d.sales_sats,
                count: d.count,
            })
            .collect(),
    }))
}
