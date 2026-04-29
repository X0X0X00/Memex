use crate::error::AppResult;
use rusqlite::Connection;
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct LengthBucket {
    pub label: String,
    pub n: i64,
}

const BUCKETS: &[(&str, i64, i64)] = &[
    ("0-50", 0, 50),
    ("50-200", 50, 200),
    ("200-1k", 200, 1_000),
    ("1k-5k", 1_000, 5_000),
    ("5k+", 5_000, i64::MAX),
];

pub fn compute(conn: &Connection) -> AppResult<Vec<LengthBucket>> {
    let mut stmt = conn.prepare(
        "SELECT COALESCE(tok_input, 0) + COALESCE(tok_output, 0) AS toks
         FROM messages
         WHERE role = 'user'",
    )?;
    let rows = stmt.query_map([], |r| r.get::<_, i64>(0))?;
    let mut counts = vec![0i64; BUCKETS.len()];
    for row in rows {
        let toks = row?;
        for (i, (_, lo, hi)) in BUCKETS.iter().enumerate() {
            if toks >= *lo && toks < *hi {
                counts[i] += 1;
                break;
            }
        }
    }
    Ok(BUCKETS
        .iter()
        .zip(counts.into_iter())
        .map(|((label, _, _), n)| LengthBucket {
            label: label.to_string(),
            n,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::*;
    use crate::schema::*;

    fn user_msg(id: &str, toks: u64) -> Message {
        Message {
            id: id.into(),
            conversation_id: "c1".into(),
            role: Role::User,
            content: "x".into(),
            timestamp: Some(0),
            model: None,
            tokens: Some(TokenCounts {
                input: toks,
                output: 0,
                cache_read: 0,
                cache_write: 0,
            }),
            tool_name: None,
        }
    }

    #[test]
    fn fills_correct_buckets() {
        let conn = open_in_memory().unwrap();
        upsert_conversation(
            &conn,
            &Conversation {
                id: "c1".into(),
                source: Source::ClaudeCode,
                native_id: "x".into(),
                title: "t".into(),
                created_at: 0,
                updated_at: 0,
                model: None,
                project: None,
                message_count: 0,
                tokens: TokenCounts::zero(),
                estimated_cost_usd: 0.0,
            },
        )
        .unwrap();
        for (i, toks) in [10u64, 30, 60, 150, 400, 2_000, 9_999].iter().enumerate() {
            insert_message(&conn, i as u32, &user_msg(&format!("m{i}"), *toks)).unwrap();
        }
        let r = compute(&conn).unwrap();
        let by: std::collections::HashMap<&str, i64> =
            r.iter().map(|b| (b.label.as_str(), b.n)).collect();
        assert_eq!(by["0-50"], 2); // 10, 30
        assert_eq!(by["50-200"], 2); // 60, 150
        assert_eq!(by["200-1k"], 1); // 400
        assert_eq!(by["1k-5k"], 1); // 2000
        assert_eq!(by["5k+"], 1); // 9999
    }

    #[test]
    fn empty_db_returns_zeroed_buckets() {
        let conn = open_in_memory().unwrap();
        let r = compute(&conn).unwrap();
        assert_eq!(r.len(), 5);
        for b in &r {
            assert_eq!(b.n, 0);
        }
    }
}
