use crate::error::AppResult;
use rusqlite::Connection;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ProjectRow {
    pub project: String,
    pub display_name: String,
    pub conv_count: i64,
    pub msg_count: i64,
    pub token_count: i64,
    pub cost_usd: f64,
    pub first_at: i64,
    pub last_at: i64,
}

pub fn compute(conn: &Connection) -> AppResult<Vec<ProjectRow>> {
    let mut stmt = conn.prepare(
        "SELECT project,
                COUNT(*) AS conv_count,
                COALESCE(SUM(message_count), 0) AS msg_count,
                COALESCE(SUM(tok_input + tok_output + tok_cache_read + tok_cache_write), 0) AS token_count,
                COALESCE(SUM(estimated_cost_usd), 0.0) AS cost,
                COALESCE(MIN(created_at), 0) AS first_at,
                COALESCE(MAX(updated_at), 0) AS last_at
         FROM conversations
         WHERE source = 'claude_code' AND project IS NOT NULL AND project != ''
         GROUP BY project
         ORDER BY cost DESC, msg_count DESC
         LIMIT 100",
    )?;
    let rows = stmt.query_map([], |r| {
        let project: String = r.get(0)?;
        let display_name = display_name_for(&project);
        Ok(ProjectRow {
            project,
            display_name,
            conv_count: r.get(1)?,
            msg_count: r.get(2)?,
            token_count: r.get(3)?,
            cost_usd: r.get(4)?,
            first_at: r.get(5)?,
            last_at: r.get(6)?,
        })
    })?;
    Ok(rows.filter_map(Result::ok).collect())
}

fn display_name_for(path: &str) -> String {
    path.rsplit('/')
        .find(|s| !s.is_empty())
        .map(String::from)
        .unwrap_or_else(|| path.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::*;
    use crate::schema::*;

    fn seed(conn: &Connection) {
        for (i, (proj, cost)) in [
            ("/Users/zzh/Visual Studio Code/Memex", 200.0),
            ("/Users/zzh/Visual Studio Code/Memex", 32.5),
            ("/Users/zzh/projects/other", 12.0),
        ]
        .iter()
        .enumerate()
        {
            upsert_conversation(
                conn,
                &Conversation {
                    id: format!("c{i}"),
                    source: Source::ClaudeCode,
                    native_id: format!("x{i}"),
                    title: "t".into(),
                    created_at: 1700000000 + (i as i64) * 1000,
                    updated_at: 1700000000 + (i as i64) * 2000,
                    model: None,
                    project: Some((*proj).into()),
                    message_count: 5,
                    tokens: TokenCounts {
                        input: 1000,
                        output: 2000,
                        cache_read: 0,
                        cache_write: 0,
                    },
                    estimated_cost_usd: *cost,
                },
            )
            .unwrap();
        }
    }

    #[test]
    fn rolls_up_per_project_sorted_by_cost() {
        let conn = open_in_memory().unwrap();
        seed(&conn);
        let r = compute(&conn).unwrap();
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].display_name, "Memex");
        assert!((r[0].cost_usd - 232.5).abs() < 0.01);
        assert_eq!(r[0].conv_count, 2);
        assert_eq!(r[1].display_name, "other");
    }

    #[test]
    fn ignores_non_cc_sources() {
        let conn = open_in_memory().unwrap();
        upsert_conversation(
            &conn,
            &Conversation {
                id: "openai-1".into(),
                source: Source::Openai,
                native_id: "x".into(),
                title: "t".into(),
                created_at: 0,
                updated_at: 0,
                model: None,
                project: Some("/should/be/ignored".into()),
                message_count: 1,
                tokens: TokenCounts::zero(),
                estimated_cost_usd: 99.0,
            },
        )
        .unwrap();
        let r = compute(&conn).unwrap();
        assert!(r.is_empty());
    }
}
