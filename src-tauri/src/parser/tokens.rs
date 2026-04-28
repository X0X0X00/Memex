//! Token counting helper. Uses tiktoken cl100k_base, which is OpenAI's tokenizer.
//! For Claude content, this is approximate (Anthropic's tokenizer isn't public),
//! but the per-character ratio is close enough for cost estimation.

use once_cell::sync::Lazy;
use tiktoken_rs::{cl100k_base, CoreBPE};

static BPE: Lazy<CoreBPE> = Lazy::new(|| cl100k_base().expect("cl100k bpe"));

pub fn count_tokens(s: &str) -> u64 {
    BPE.encode_with_special_tokens(s).len() as u64
}
