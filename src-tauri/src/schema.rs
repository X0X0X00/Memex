use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Openai,
    ClaudeWeb,
    ClaudeCode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    System,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenCounts {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
}

impl TokenCounts {
    pub fn zero() -> Self {
        Self { input: 0, output: 0, cache_read: 0, cache_write: 0 }
    }
    pub fn total(&self) -> u64 {
        self.input + self.output + self.cache_read + self.cache_write
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub source: Source,
    pub native_id: String,
    pub title: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub model: Option<String>,
    pub project: Option<String>,
    pub message_count: u32,
    pub tokens: TokenCounts,
    pub estimated_cost_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub conversation_id: String,
    pub role: Role,
    pub content: String,
    pub timestamp: Option<i64>,
    pub model: Option<String>,
    pub tokens: Option<TokenCounts>,
    pub tool_name: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_total_sums_all_categories() {
        let t = TokenCounts { input: 10, output: 20, cache_read: 30, cache_write: 40 };
        assert_eq!(t.total(), 100);
    }

    #[test]
    fn source_serializes_to_snake_case() {
        let s = serde_json::to_string(&Source::ClaudeCode).unwrap();
        assert_eq!(s, "\"claude_code\"");
    }

    #[test]
    fn role_serializes_to_lowercase() {
        let s = serde_json::to_string(&Role::Assistant).unwrap();
        assert_eq!(s, "\"assistant\"");
    }
}
