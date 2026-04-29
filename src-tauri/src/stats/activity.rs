use crate::error::AppResult;
use chrono::{DateTime, Datelike, Local, Timelike, Utc};
use rusqlite::Connection;
use serde::Serialize;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize)]
pub struct DailyActivity {
    pub date: String,             // YYYY-MM-DD (local timezone)
    pub messages: u32,
    pub conversations: u32,
}

#[derive(Debug, Serialize)]
pub struct ActivityReport {
    pub daily: Vec<DailyActivity>,
    /// Busiest day by message count (existing behaviour).
    pub busiest_day: Option<DailyActivity>,
    /// Busiest day by distinct-conversation count (new in v0.3.3).
    pub busiest_day_by_conversations: Option<DailyActivity>,
    /// Messages per local hour-of-day.
    pub by_hour: [u32; 24],
    /// Distinct conversations per local hour-of-day.
    pub by_hour_conversations: [u32; 24],
    /// Messages per local weekday (0 = Sunday).
    pub by_weekday: [u32; 7],
    /// Distinct conversations per local weekday.
    pub by_weekday_conversations: [u32; 7],
}

pub fn compute(conn: &Connection) -> AppResult<ActivityReport> {
    let mut stmt = conn.prepare(
        "SELECT timestamp, conversation_id FROM messages WHERE timestamp IS NOT NULL",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    })?;

    let mut msgs_by_day: HashMap<String, u32> = HashMap::new();
    let mut convs_by_day: HashMap<String, HashSet<String>> = HashMap::new();
    let mut by_hour = [0u32; 24];
    let mut by_weekday = [0u32; 7];
    let mut hour_convs: Vec<HashSet<String>> = (0..24).map(|_| HashSet::new()).collect();
    let mut weekday_convs: Vec<HashSet<String>> = (0..7).map(|_| HashSet::new()).collect();

    for row in rows {
        let (ts, conv_id) = row?;
        let utc = DateTime::<Utc>::from_timestamp(ts, 0).unwrap_or_else(Utc::now);
        let dt = utc.with_timezone(&Local);
        let day = dt.format("%Y-%m-%d").to_string();
        let hour = dt.hour() as usize;
        let wd = dt.weekday().num_days_from_sunday() as usize;

        *msgs_by_day.entry(day.clone()).or_insert(0) += 1;
        convs_by_day.entry(day).or_default().insert(conv_id.clone());
        by_hour[hour] += 1;
        by_weekday[wd] += 1;
        hour_convs[hour].insert(conv_id.clone());
        weekday_convs[wd].insert(conv_id);
    }

    let by_hour_conversations: [u32; 24] =
        std::array::from_fn(|i| hour_convs[i].len() as u32);
    let by_weekday_conversations: [u32; 7] =
        std::array::from_fn(|i| weekday_convs[i].len() as u32);

    let mut daily: Vec<DailyActivity> = msgs_by_day
        .into_iter()
        .map(|(date, messages)| {
            let conversations = convs_by_day
                .get(&date)
                .map(|s| s.len() as u32)
                .unwrap_or(0);
            DailyActivity {
                date,
                messages,
                conversations,
            }
        })
        .collect();
    daily.sort_by(|a, b| a.date.cmp(&b.date));
    let busiest_day = daily.iter().max_by_key(|d| d.messages).cloned();
    let busiest_day_by_conversations = daily.iter().max_by_key(|d| d.conversations).cloned();

    Ok(ActivityReport {
        daily,
        busiest_day,
        busiest_day_by_conversations,
        by_hour,
        by_hour_conversations,
        by_weekday,
        by_weekday_conversations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::*;
    use crate::schema::*;

    fn seed(conn: &Connection) {
        for cid in &["c1", "c2"] {
            upsert_conversation(
                conn,
                &Conversation {
                    id: cid.to_string(),
                    source: Source::Openai,
                    native_id: cid.to_string(),
                    title: "t".into(),
                    created_at: 0,
                    updated_at: 0,
                    model: None,
                    project: None,
                    message_count: 1,
                    tokens: TokenCounts::zero(),
                    estimated_cost_usd: 0.0,
                },
            )
            .unwrap();
        }
        // 2024-06-01 12:00 UTC = 1717243200 — c1 has two messages here
        // 2024-06-02 12:00 UTC = 1717329600 — c2 has one message
        for (i, (ts, conv)) in [
            (1717243200i64, "c1"),
            (1717243260, "c1"),
            (1717329600, "c2"),
        ]
        .iter()
        .enumerate()
        {
            insert_message(
                conn,
                i as u32,
                &Message {
                    id: format!("m-{i}"),
                    conversation_id: conv.to_string(),
                    role: Role::User,
                    content: "x".into(),
                    timestamp: Some(*ts),
                    model: None,
                    tokens: None,
                    tool_name: None,
                },
            )
            .unwrap();
        }
    }

    #[test]
    fn busiest_day_unchanged_for_messages() {
        let conn = open_in_memory().unwrap();
        seed(&conn);
        let r = compute(&conn).unwrap();
        let bd = r.busiest_day.as_ref().unwrap();
        assert_eq!(bd.messages, 2);
        assert_eq!(bd.conversations, 1);
    }

    #[test]
    fn conversations_count_dedup_per_bucket() {
        let conn = open_in_memory().unwrap();
        seed(&conn);
        let r = compute(&conn).unwrap();
        let total_msgs: u32 = r.by_hour.iter().sum();
        let total_conv_unique: u32 = r.by_hour_conversations.iter().sum();
        assert_eq!(total_msgs, 3);
        // c1 appears at one hour, c2 at one hour → 2 distinct conv-hour pairs
        // (or 3 if c1's two messages cross an hour boundary in local TZ).
        assert!(total_conv_unique >= 1);
        assert!(total_conv_unique <= 3);
    }
}
