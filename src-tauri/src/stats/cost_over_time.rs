use crate::error::AppResult;
use crate::schema::TokenCounts;
use crate::stats::cost::estimate_cost_usd;
use chrono::{DateTime, Datelike, Local, Utc};
use rusqlite::Connection;
use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BucketKind {
    Daily,
    Weekly,
    Monthly,
}

#[derive(Debug, Serialize)]
pub struct CostPoint {
    pub bucket_label: String,
    pub by_model_group: Vec<(String, f64)>,
}

#[derive(Debug, Serialize)]
pub struct CostSeries {
    pub bucket: BucketKind,
    pub points: Vec<CostPoint>,
}

pub fn pick_bucket(span_days: i64) -> BucketKind {
    if span_days <= 60 {
        BucketKind::Daily
    } else if span_days <= 365 {
        BucketKind::Weekly
    } else {
        BucketKind::Monthly
    }
}

pub fn model_group(model: &str) -> &'static str {
    let m = model.to_lowercase();
    if m.starts_with("claude-opus") {
        "Claude Opus"
    } else if m.starts_with("claude-sonnet")
        || m.starts_with("claude-3-5-sonnet")
        || m.starts_with("claude-3-7-sonnet")
    {
        "Claude Sonnet"
    } else if m.starts_with("claude-haiku") || m.starts_with("claude-3-5-haiku") {
        "Claude Haiku"
    } else if m.starts_with("claude") {
        "Claude (other)"
    } else if m.starts_with("gpt-4o") {
        "GPT-4o"
    } else if m.starts_with("gpt-4") {
        "GPT-4"
    } else if m.starts_with("gpt-3.5") {
        "GPT-3.5"
    } else if m.starts_with("o1") || m.starts_with("o3") {
        "o-series"
    } else if m.is_empty() {
        "(unknown)"
    } else {
        "Other"
    }
}

pub fn compute(conn: &Connection) -> AppResult<CostSeries> {
    let span: Option<(i64, i64)> = conn
        .query_row(
            "SELECT MIN(created_at), MAX(created_at) FROM conversations WHERE created_at > 0",
            [],
            |r| {
                Ok((
                    r.get::<_, i64>(0).unwrap_or(0),
                    r.get::<_, i64>(1).unwrap_or(0),
                ))
            },
        )
        .ok();

    let (min_ts, max_ts) = match span {
        Some((a, b)) if a > 0 && b >= a => (a, b),
        _ => {
            return Ok(CostSeries {
                bucket: BucketKind::Monthly,
                points: Vec::new(),
            })
        }
    };
    let span_days = (max_ts - min_ts) / 86_400;
    let bucket = pick_bucket(span_days);

    let mut stmt = conn.prepare(
        "SELECT m.timestamp,
                COALESCE(m.model, c.model, '') AS model,
                COALESCE(m.tok_input, 0),
                COALESCE(m.tok_output, 0),
                COALESCE(m.tok_cache_read, 0),
                COALESCE(m.tok_cache_write, 0)
         FROM messages m
         JOIN conversations c ON c.id = m.conversation_id
         WHERE m.timestamp IS NOT NULL",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, i64>(2)?,
            r.get::<_, i64>(3)?,
            r.get::<_, i64>(4)?,
            r.get::<_, i64>(5)?,
        ))
    })?;

    use std::collections::BTreeMap;
    let mut acc: BTreeMap<String, BTreeMap<String, f64>> = BTreeMap::new();
    for row in rows {
        let (ts, model, ti, to, tcr, tcw) = row?;
        let label = bucket_label(ts, bucket);
        let group = model_group(&model).to_string();
        let toks = TokenCounts {
            input: ti as u64,
            output: to as u64,
            cache_read: tcr as u64,
            cache_write: tcw as u64,
        };
        let cost = estimate_cost_usd(&model, &toks);
        if cost > 0.0 {
            *acc.entry(label).or_default().entry(group).or_insert(0.0) += cost;
        }
    }

    let points = acc
        .into_iter()
        .map(|(bucket_label, by_group)| CostPoint {
            bucket_label,
            by_model_group: by_group.into_iter().collect(),
        })
        .collect();

    Ok(CostSeries { bucket, points })
}

fn bucket_label(unix_ts: i64, bucket: BucketKind) -> String {
    let dt = DateTime::<Utc>::from_timestamp(unix_ts, 0)
        .unwrap_or_else(Utc::now)
        .with_timezone(&Local);
    match bucket {
        BucketKind::Daily => dt.format("%Y-%m-%d").to_string(),
        BucketKind::Weekly => format!("{}-W{:02}", dt.iso_week().year(), dt.iso_week().week()),
        BucketKind::Monthly => dt.format("%Y-%m").to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_daily_for_short_span() {
        assert!(matches!(pick_bucket(30), BucketKind::Daily));
    }
    #[test]
    fn picks_weekly_for_medium_span() {
        assert!(matches!(pick_bucket(180), BucketKind::Weekly));
    }
    #[test]
    fn picks_monthly_for_long_span() {
        assert!(matches!(pick_bucket(800), BucketKind::Monthly));
    }

    #[test]
    fn model_group_buckets_correctly() {
        assert_eq!(model_group("claude-opus-4-7"), "Claude Opus");
        assert_eq!(model_group("claude-sonnet-4-6"), "Claude Sonnet");
        assert_eq!(model_group("claude-3-5-sonnet-20241022"), "Claude Sonnet");
        assert_eq!(model_group("claude-haiku-4-5-20251001"), "Claude Haiku");
        assert_eq!(model_group("gpt-4o-mini"), "GPT-4o");
        assert_eq!(model_group("gpt-4-turbo"), "GPT-4");
        assert_eq!(model_group(""), "(unknown)");
        assert_eq!(model_group("mistral-large"), "Other");
    }

    #[test]
    fn compute_returns_empty_when_no_data() {
        let conn = crate::db::open_in_memory().unwrap();
        let r = compute(&conn).unwrap();
        assert!(r.points.is_empty());
    }

    #[test]
    fn compute_aggregates_by_model_group_and_bucket() {
        use crate::db::*;
        use crate::schema::*;

        let conn = open_in_memory().unwrap();
        upsert_conversation(
            &conn,
            &Conversation {
                id: "c1".into(),
                source: Source::ClaudeCode,
                native_id: "x".into(),
                title: "t".into(),
                created_at: 1700000000,
                updated_at: 1700100000,
                model: Some("claude-opus-4-7".into()),
                project: None,
                message_count: 2,
                tokens: TokenCounts::zero(),
                estimated_cost_usd: 0.0,
            },
        )
        .unwrap();
        for (i, ts) in [1700000000i64, 1700000060].iter().enumerate() {
            insert_message(
                &conn,
                i as u32,
                &Message {
                    id: format!("m-{i}"),
                    conversation_id: "c1".into(),
                    role: Role::Assistant,
                    content: "x".into(),
                    timestamp: Some(*ts),
                    model: Some("claude-opus-4-7".into()),
                    tokens: Some(TokenCounts {
                        input: 0,
                        output: 1_000_000,
                        cache_read: 0,
                        cache_write: 0,
                    }),
                    tool_name: None,
                },
            )
            .unwrap();
        }
        let r = compute(&conn).unwrap();
        assert_eq!(r.points.len(), 1);
        let p = &r.points[0];
        assert_eq!(p.by_model_group.len(), 1);
        assert_eq!(p.by_model_group[0].0, "Claude Opus");
        // 1M output * 2 messages * $75/M = $150
        assert!((p.by_model_group[0].1 - 150.0).abs() < 0.01);
    }
}
