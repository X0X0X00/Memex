use crate::error::AppResult;
use chrono::{DateTime, Datelike, Timelike, Utc};
use rusqlite::Connection;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct DailyActivity {
    pub date: String,    // YYYY-MM-DD (UTC)
    pub messages: u32,
}

#[derive(Debug, Serialize)]
pub struct ActivityReport {
    pub daily: Vec<DailyActivity>,
    pub busiest_day: Option<DailyActivity>,
    pub by_hour: [u32; 24],
    pub by_weekday: [u32; 7],   // 0 = Sunday
}

pub fn compute(conn: &Connection) -> AppResult<ActivityReport> {
    let mut stmt = conn.prepare(
        "SELECT timestamp FROM messages WHERE timestamp IS NOT NULL"
    )?;
    let mut by_day: HashMap<String, u32> = HashMap::new();
    let mut by_hour = [0u32; 24];
    let mut by_weekday = [0u32; 7];
    let rows = stmt.query_map([], |row| row.get::<_, i64>(0))?;
    for ts in rows {
        let ts = ts?;
        let dt = DateTime::<Utc>::from_timestamp(ts, 0).unwrap_or_else(Utc::now);
        let day = dt.format("%Y-%m-%d").to_string();
        *by_day.entry(day).or_insert(0) += 1;
        by_hour[dt.hour() as usize] += 1;
        by_weekday[dt.weekday().num_days_from_sunday() as usize] += 1;
    }
    let mut daily: Vec<_> = by_day.into_iter()
        .map(|(date, messages)| DailyActivity { date, messages })
        .collect();
    daily.sort_by(|a, b| a.date.cmp(&b.date));
    let busiest_day = daily.iter().max_by_key(|d| d.messages).cloned();
    Ok(ActivityReport { daily, busiest_day, by_hour, by_weekday })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::*;
    use crate::schema::*;

    fn seed(conn: &Connection) {
        let conv = Conversation {
            id: "c1".into(), source: Source::Openai, native_id: "x".into(),
            title: "t".into(), created_at: 0, updated_at: 0, model: None, project: None,
            message_count: 3, tokens: TokenCounts::zero(), estimated_cost_usd: 0.0,
        };
        upsert_conversation(conn, &conv).unwrap();
        // 2024-06-01 12:00 UTC = 1717243200
        for (i, ts) in [1717243200i64, 1717243260, 1717329600].iter().enumerate() {
            insert_message(conn, i as u32, &Message {
                id: format!("m-{i}"), conversation_id: "c1".into(),
                role: Role::User, content: "x".into(),
                timestamp: Some(*ts), model: None, tokens: None, tool_name: None,
            }).unwrap();
        }
    }

    #[test]
    fn busiest_day_is_first() {
        let conn = open_in_memory().unwrap();
        seed(&conn);
        let r = compute(&conn).unwrap();
        assert_eq!(r.busiest_day.as_ref().unwrap().date, "2024-06-01");
        assert_eq!(r.busiest_day.as_ref().unwrap().messages, 2);
        assert_eq!(r.daily.len(), 2);
    }
}
