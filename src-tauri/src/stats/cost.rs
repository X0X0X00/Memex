use crate::schema::TokenCounts;

/// USD per 1M tokens.
#[derive(Debug, Clone, Copy)]
pub struct ModelPrice {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
}

pub fn price_for(model: &str) -> Option<ModelPrice> {
    let m = model.to_lowercase();
    // Claude
    if m.starts_with("claude-opus-4-7") || m.starts_with("claude-opus-4") {
        return Some(ModelPrice { input: 15.0, output: 75.0, cache_read: 1.5, cache_write: 18.75 });
    }
    if m.starts_with("claude-sonnet-4-6")
        || m.starts_with("claude-sonnet-4")
        || m.starts_with("claude-3-5-sonnet")
        || m.starts_with("claude-3-7-sonnet")
    {
        return Some(ModelPrice { input: 3.0, output: 15.0, cache_read: 0.3, cache_write: 3.75 });
    }
    if m.starts_with("claude-haiku-4") || m.starts_with("claude-3-5-haiku") {
        return Some(ModelPrice { input: 0.8, output: 4.0, cache_read: 0.08, cache_write: 1.0 });
    }
    if m.starts_with("claude") {
        // Generic fallback for unknown Claude family — assume Sonnet pricing.
        return Some(ModelPrice { input: 3.0, output: 15.0, cache_read: 0.3, cache_write: 3.75 });
    }
    // OpenAI: GPT-5 family (Aug 2025+, ChatGPT app slugs include gpt-5, gpt-5-2, gpt-5-thinking, etc.)
    if m.starts_with("gpt-5-thinking") || m.starts_with("gpt-5-pro") {
        // Reasoning variants billed higher
        return Some(ModelPrice { input: 5.0, output: 40.0, cache_read: 0.5, cache_write: 0.0 });
    }
    if m.starts_with("gpt-5-mini") || m.starts_with("gpt-5-nano") {
        return Some(ModelPrice { input: 0.25, output: 2.0, cache_read: 0.025, cache_write: 0.0 });
    }
    if m.starts_with("gpt-5") {
        // Standard GPT-5
        return Some(ModelPrice { input: 1.25, output: 10.0, cache_read: 0.125, cache_write: 0.0 });
    }
    // OpenAI: GPT-4 family
    if m.starts_with("gpt-4o-mini") {
        return Some(ModelPrice { input: 0.15, output: 0.6, cache_read: 0.075, cache_write: 0.0 });
    }
    if m.starts_with("gpt-4o") || m == "gpt-4-turbo" {
        return Some(ModelPrice { input: 2.5, output: 10.0, cache_read: 1.25, cache_write: 0.0 });
    }
    if m.starts_with("gpt-4-gizmo") {
        // Custom GPTs run on the underlying GPT-4 model — price like gpt-4o.
        return Some(ModelPrice { input: 2.5, output: 10.0, cache_read: 1.25, cache_write: 0.0 });
    }
    if m.starts_with("gpt-4") {
        return Some(ModelPrice { input: 30.0, output: 60.0, cache_read: 0.0, cache_write: 0.0 });
    }
    if m.starts_with("gpt-3.5") || m.starts_with("text-davinci") {
        return Some(ModelPrice { input: 0.5, output: 1.5, cache_read: 0.0, cache_write: 0.0 });
    }
    // o-series reasoning models
    if m.starts_with("o3-mini") {
        return Some(ModelPrice { input: 1.1, output: 4.4, cache_read: 0.55, cache_write: 0.0 });
    }
    if m.starts_with("o1-mini") {
        return Some(ModelPrice { input: 1.1, output: 4.4, cache_read: 0.55, cache_write: 0.0 });
    }
    if m.starts_with("o1") || m.starts_with("o3") {
        return Some(ModelPrice { input: 15.0, output: 60.0, cache_read: 7.5, cache_write: 0.0 });
    }
    // Generic GPT fallback — assume gpt-4o for unknown OpenAI models
    if m.starts_with("gpt") {
        return Some(ModelPrice { input: 2.5, output: 10.0, cache_read: 1.25, cache_write: 0.0 });
    }
    None
}

pub fn estimate_cost_usd(model: &str, t: &TokenCounts) -> f64 {
    let p = match price_for(model) { Some(p) => p, None => return 0.0 };
    let m = 1_000_000.0;
    (t.input as f64) * p.input / m
        + (t.output as f64) * p.output / m
        + (t.cache_read as f64) * p.cache_read / m
        + (t.cache_write as f64) * p.cache_write / m
}

/// API-equivalent cost for a web export conversation.
///
/// Web exports (ChatGPT, Claude.ai) only record per-message *content* tokens.
/// Real APIs bill the cumulative input on every assistant turn, so
/// a 20-turn chat costs much more than naive content-token sum.
///
/// This function walks messages in order, maintaining a running input
/// total. On each assistant turn it bills (running_input × input_rate)
/// + (output × output_rate). Per-message model is preferred; fall back
/// to `default_model` (e.g. "gpt-4o") if the message has none recorded.
///
/// For Claude Code (`source = ClaudeCode`) the API response gives exact
/// per-call usage including cache reads, so callers should NOT use this
/// function — they should sum `estimate_cost_usd` per message instead.
pub fn cumulative_billing_cost(
    messages: &[crate::schema::Message],
    default_model: &str,
) -> f64 {
    use crate::schema::{Role, TokenCounts as TC};
    let mut running_input: u64 = 0;
    let mut total = 0.0;
    for m in messages {
        let tokens = match &m.tokens { Some(t) => t, None => continue };
        let model = m.model.as_deref().unwrap_or(default_model);
        match m.role {
            Role::Assistant => {
                // Bill (cumulative input + this output) for the turn.
                let billed = TC {
                    input: running_input,
                    output: tokens.output,
                    cache_read: 0,
                    cache_write: 0,
                };
                total += estimate_cost_usd(model, &billed);
                running_input += tokens.output;
            }
            _ => {
                running_input += tokens.input;
            }
        }
    }
    total
}

#[cfg(test)]
mod cumulative_tests {
    use super::*;
    use crate::schema::*;

    fn user(i: u32, toks: u64) -> Message {
        Message {
            id: format!("u{i}"), conversation_id: "c".into(), role: Role::User,
            content: "x".into(), timestamp: Some(0), model: None,
            tokens: Some(TokenCounts { input: toks, output: 0, cache_read: 0, cache_write: 0 }),
            tool_name: None,
        }
    }
    fn asst(i: u32, toks: u64, model: &str) -> Message {
        Message {
            id: format!("a{i}"), conversation_id: "c".into(), role: Role::Assistant,
            content: "x".into(), timestamp: Some(0), model: Some(model.into()),
            tokens: Some(TokenCounts { input: 0, output: toks, cache_read: 0, cache_write: 0 }),
            tool_name: None,
        }
    }

    #[test]
    fn single_turn_billed_like_naive() {
        // u(100) → a(800) on gpt-4o: input 100, output 800
        // gpt-4o: $2.5/M in, $10/M out → 0.00025 + 0.008 = $0.00825
        let msgs = vec![user(0, 100), asst(0, 800, "gpt-4o")];
        let c = cumulative_billing_cost(&msgs, "gpt-4o");
        assert!((c - 0.00825).abs() < 1e-6, "got {c}");
    }

    #[test]
    fn multi_turn_input_compounds() {
        // 3 turns each user 100 / asst 200, on gpt-4o
        // Turn 1: input=100, output=200
        // Turn 2: input = 100+200+100 = 400, output=200
        // Turn 3: input = 400+200+100 = 700, output=200
        // total input billed = 1200, total output = 600
        // cost = 1200*$2.5/M + 600*$10/M = 0.003 + 0.006 = $0.009
        let mut msgs = vec![];
        for i in 0..3 {
            msgs.push(user(i, 100));
            msgs.push(asst(i, 200, "gpt-4o"));
        }
        let c = cumulative_billing_cost(&msgs, "gpt-4o");
        assert!((c - 0.009).abs() < 1e-6, "got {c}");
    }

    #[test]
    fn naive_vs_cumulative_diverges_for_long_chats() {
        // 10 turns on gpt-4o
        let mut msgs = vec![];
        for i in 0..10 {
            msgs.push(user(i, 100));
            msgs.push(asst(i, 800, "gpt-4o"));
        }
        let cumulative = cumulative_billing_cost(&msgs, "gpt-4o");
        // Naive: input total 1000 + output total 8000, single bill
        let naive = estimate_cost_usd(
            "gpt-4o",
            &TokenCounts { input: 1000, output: 8000, cache_read: 0, cache_write: 0 },
        );
        // For 10-turn output-heavy chats on gpt-4o the ratio works out to
        // ~2.2x (output dominates the bill, but input compounds quadratically).
        // Output-heavy ratio is the lower bound; input-heavy chats reach 5-10x.
        assert!(cumulative > naive * 2.0, "cumulative {cumulative} vs naive {naive}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_model_zero_cost() {
        let t = TokenCounts { input: 1000, output: 1000, cache_read: 0, cache_write: 0 };
        assert_eq!(estimate_cost_usd("foo-bar", &t), 0.0);
    }

    #[test]
    fn gpt4o_million_io_costs_right() {
        let t = TokenCounts { input: 1_000_000, output: 1_000_000, cache_read: 0, cache_write: 0 };
        let c = estimate_cost_usd("gpt-4o", &t);
        assert!((c - 12.5).abs() < 0.001, "got {c}");
    }

    #[test]
    fn claude_sonnet_matches_table() {
        let t = TokenCounts { input: 1_000_000, output: 1_000_000, cache_read: 1_000_000, cache_write: 1_000_000 };
        let c = estimate_cost_usd("claude-sonnet-4-6", &t);
        assert!((c - (3.0 + 15.0 + 0.3 + 3.75)).abs() < 0.001);
    }

    #[test]
    fn claude_unknown_falls_back_to_sonnet() {
        let t = TokenCounts { input: 1_000_000, output: 0, cache_read: 0, cache_write: 0 };
        let c = estimate_cost_usd("claude-mystery-model", &t);
        assert!((c - 3.0).abs() < 0.001);
    }
}
