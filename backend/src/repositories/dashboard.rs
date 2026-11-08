use chrono::NaiveDate;
use sqlx::PgPool;
use uuid::Uuid;

/// Totals over real (non-demo) payments. "Today" is in the merchant's time zone.
#[derive(Debug, sqlx::FromRow)]
pub struct SummaryRow {
    pub total_sales_sats: i64,
    pub paid_count: i64,
    pub pending_count: i64,
    pub today_sales_sats: i64,
    pub today_count: i64,
}

#[derive(Debug, sqlx::FromRow)]
pub struct DayRow {
    pub day: NaiveDate,
    pub sales_sats: i64,
    pub count: i64,
}

pub async fn summary(db: &PgPool, merchant_id: Uuid, tz: &str) -> sqlx::Result<SummaryRow> {
    sqlx::query_as::<_, SummaryRow>(
        "WITH bounds AS (SELECT date_trunc('day', now() AT TIME ZONE $2) AT TIME ZONE $2 AS today_start)
         SELECT
           COALESCE(SUM(amount_sats) FILTER (WHERE status = 'paid'), 0)::BIGINT AS total_sales_sats,
           COUNT(*) FILTER (WHERE status = 'paid') AS paid_count,
           COUNT(*) FILTER (WHERE status = 'pending') AS pending_count,
           COALESCE(SUM(amount_sats) FILTER (WHERE status = 'paid' AND settled_at >= b.today_start), 0)::BIGINT
             AS today_sales_sats,
           COUNT(*) FILTER (WHERE status = 'paid' AND settled_at >= b.today_start) AS today_count
         FROM bounds b
         LEFT JOIN payments p ON p.merchant_id = $1 AND p.is_demo = false
         GROUP BY b.today_start",
    )
    .bind(merchant_id)
    .bind(tz)
    .fetch_one(db)
    .await
}

/// Paid sales per day for the last `days` days (including today), zero-filled.
pub async fn daily_sales(
    db: &PgPool,
    merchant_id: Uuid,
    tz: &str,
    days: i32,
) -> sqlx::Result<Vec<DayRow>> {
    sqlx::query_as::<_, DayRow>(
        "WITH days AS (
           SELECT generate_series(
             (now() AT TIME ZONE $2)::date - ($3 - 1),
             (now() AT TIME ZONE $2)::date,
             interval '1 day')::date AS day
         )
         SELECT d.day,
                COALESCE(SUM(p.amount_sats), 0)::BIGINT AS sales_sats,
                COUNT(p.id) AS count
         FROM days d
         LEFT JOIN payments p
           ON p.merchant_id = $1 AND p.status = 'paid' AND p.is_demo = false
          AND (p.settled_at AT TIME ZONE $2)::date = d.day
         GROUP BY d.day ORDER BY d.day",
    )
    .bind(merchant_id)
    .bind(tz)
    .bind(days)
    .fetch_all(db)
    .await
}

/// True if Postgres knows this IANA time zone name.
pub async fn is_known_time_zone(db: &PgPool, tz: &str) -> sqlx::Result<bool> {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_timezone_names WHERE name = $1)")
        .bind(tz)
        .fetch_one(db)
        .await
}
