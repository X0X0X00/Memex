use crate::error::AppResult;
use rusqlite::Connection;

pub fn compute(conn: &Connection, k: usize) -> AppResult<Vec<(String, i64)>> {
    let mut stmt = conn.prepare(
        "SELECT tool_name, COUNT(*) AS n
         FROM tool_calls
         GROUP BY tool_name
         ORDER BY n DESC
         LIMIT ?1",
    )?;
    let rows = stmt.query_map([k as i64], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
    })?;
    Ok(rows.filter_map(Result::ok).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::*;
    use crate::schema::*;

    fn seed_msg(conn: &Connection) {
        upsert_conversation(
            conn,
            &Conversation {
                id: "c1".into(),
                source: Source::ClaudeCode,
                native_id: "x".into(),
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
        insert_message(
            conn,
            0,
            &Message {
                id: "m1".into(),
                conversation_id: "c1".into(),
                role: Role::Assistant,
                content: "x".into(),
                timestamp: Some(0),
                model: None,
                tokens: None,
                tool_name: None,
            },
        )
        .unwrap();
    }

    #[test]
    fn returns_empty_when_no_tool_calls() {
        let conn = open_in_memory().unwrap();
        let r = compute(&conn, 10).unwrap();
        assert!(r.is_empty());
    }

    #[test]
    fn ranks_by_count_descending() {
        let conn = open_in_memory().unwrap();
        seed_msg(&conn);
        for (i, name) in ["Bash", "Bash", "Bash", "Read", "Read", "Edit"]
            .iter()
            .enumerate()
        {
            insert_tool_call(&conn, &format!("m1:t{i}"), "m1", "c1", name, i as u32).unwrap();
        }
        let r = compute(&conn, 10).unwrap();
        assert_eq!(
            r,
            vec![
                ("Bash".to_string(), 3),
                ("Read".to_string(), 2),
                ("Edit".to_string(), 1),
            ]
        );
    }

    #[test]
    fn limit_is_respected() {
        let conn = open_in_memory().unwrap();
        seed_msg(&conn);
        for i in 0..15 {
            insert_tool_call(
                &conn,
                &format!("m1:t{i}"),
                "m1",
                "c1",
                &format!("Tool{i}"),
                i,
            )
            .unwrap();
        }
        let r = compute(&conn, 5).unwrap();
        assert_eq!(r.len(), 5);
    }
}
