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
