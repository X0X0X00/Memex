use crate::error::AppResult;
use crate::stats::ngrams::{extract_ngrams, top_phrases, PhraseStat};
use rusqlite::Connection;
use std::collections::HashMap;

pub fn top_topics(conn: &Connection, k: usize) -> AppResult<Vec<PhraseStat>> {
    // Each "topic doc" = title + first user message of the conversation.
    let mut stmt = conn.prepare(
        "SELECT c.title,
                (SELECT m.content FROM messages m
                 WHERE m.conversation_id = c.id AND m.role = 'user'
                 ORDER BY m.seq ASC LIMIT 1) AS first_user
         FROM conversations c"
    )?;
    let docs: Vec<String> = stmt
        .query_map([], |r| {
            let title: String = r.get(0)?;
            let first: Option<String> = r.get(1)?;
            Ok(format!("{title} {}", first.unwrap_or_default()))
        })?
        .filter_map(Result::ok)
        .collect();

    let n_docs = docs.len() as f64;
    if n_docs < 2.0 {
        return Ok(top_phrases(&docs, k));
    }

    let mut doc_terms: Vec<HashMap<String, u32>> = Vec::with_capacity(docs.len());
    let mut df: HashMap<String, u32> = HashMap::new();
    for d in &docs {
        let mut tf: HashMap<String, u32> = HashMap::new();
        for ngram in extract_ngrams(d) {
            *tf.entry(ngram).or_insert(0) += 1;
        }
        for k in tf.keys() {
            *df.entry(k.clone()).or_insert(0) += 1;
        }
        doc_terms.push(tf);
    }

    let mut score: HashMap<String, f64> = HashMap::new();
    let mut count: HashMap<String, u32> = HashMap::new();
    for tf in &doc_terms {
        for (term, freq) in tf {
            let idf = ((n_docs + 1.0) / (*df.get(term).unwrap_or(&1) as f64 + 1.0)).ln() + 1.0;
            *score.entry(term.clone()).or_insert(0.0) += (*freq as f64) * idf;
            *count.entry(term.clone()).or_insert(0) += freq;
        }
    }

    let mut v: Vec<PhraseStat> = score
        .into_iter()
        .filter(|(t, _)| t.chars().count() >= 2)
        .map(|(phrase, s)| PhraseStat {
            count: *count.get(&phrase).unwrap_or(&0),
            score: s,
            phrase,
        })
        .collect();
    v.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    v.truncate(k);
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::*;
    use crate::schema::*;

    fn seed_topics(conn: &Connection) {
        for (i, (title, first)) in [
            ("rust ownership", "explain rust ownership and borrowing"),
            ("rust lifetimes", "what are lifetimes in rust"),
            ("python decorators", "how do python decorators work"),
        ].iter().enumerate() {
            let cid = format!("c{i}");
            upsert_conversation(conn, &Conversation {
                id: cid.clone(), source: Source::Openai, native_id: format!("x{i}"),
                title: title.to_string(), created_at: 0, updated_at: 0,
                model: None, project: None, message_count: 1,
                tokens: TokenCounts::zero(), estimated_cost_usd: 0.0,
            }).unwrap();
            insert_message(conn, 0, &Message {
                id: format!("m{i}"), conversation_id: cid, role: Role::User,
                content: first.to_string(), timestamp: Some(0), model: None,
                tokens: None, tool_name: None,
            }).unwrap();
        }
    }

    #[test]
    fn rust_topic_ranks_high() {
        let conn = open_in_memory().unwrap();
        seed_topics(&conn);
        let r = top_topics(&conn, 5).unwrap();
        assert!(r.iter().any(|p| p.phrase.contains("rust")), "got {:?}", r);
    }
}
