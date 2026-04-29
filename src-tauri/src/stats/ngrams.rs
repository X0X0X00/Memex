use crate::error::AppResult;
use once_cell::sync::Lazy;
use rusqlite::Connection;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct PhraseStat {
    pub phrase: String,
    pub count: u32,
    pub score: f64,
}

pub fn top_phrases_for_user(conn: &Connection, k: usize) -> AppResult<Vec<PhraseStat>> {
    let mut stmt = conn.prepare("SELECT content FROM messages WHERE role = 'user'")?;
    let texts: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(top_phrases(&texts, k))
}

pub fn top_phrases(texts: &[String], k: usize) -> Vec<PhraseStat> {
    let mut counts: HashMap<String, u32> = HashMap::new();
    for t in texts {
        for ngram in extract_ngrams(t) {
            *counts.entry(ngram).or_insert(0) += 1;
        }
    }
    let mut v: Vec<PhraseStat> = counts.into_iter()
        .filter(|(p, c)| *c >= 2 && !is_too_short_or_noisy(p))
        .map(|(p, c)| {
            let chars = p.chars().count() as f64;
            PhraseStat {
                score: (c as f64) * (1.0 + chars.ln().max(0.0)),
                phrase: p,
                count: c,
            }
        })
        .collect();
    v.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    v.truncate(k);
    v
}

fn is_too_short_or_noisy(p: &str) -> bool {
    let chars = p.chars().count();
    if chars < 2 { return true; }
    if p.chars().all(|c| !c.is_alphabetic()) { return true; }
    false
}

pub fn extract_ngrams(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    // Split into "lines" by both \n and Chinese 。/！/？ — anything paragraph-shaped.
    for line in text.split(|c: char| c == '\n' || c == '。' || c == '！' || c == '？') {
        if is_template_line(line) {
            continue;
        }
        out.extend(extract_ngrams_line(line));
    }
    out
}

fn extract_ngrams_line(text: &str) -> Vec<String> {
    let cjk = cjk_ratio(text);
    let (tokens, stop): (Vec<String>, &[&str]) = if cjk > 0.3 {
        (tokenize_zh(text), ZH_STOPWORDS)
    } else {
        (tokenize_en(text), EN_STOPWORDS)
    };
    let filtered: Vec<&str> = tokens
        .iter()
        .map(String::as_str)
        .filter(|t| !stop.contains(t))
        .collect();
    let mut out = Vec::new();
    for n in 1..=3 {
        if filtered.len() < n {
            continue;
        }
        for w in filtered.windows(n) {
            let phrase = if cjk > 0.3 { w.concat() } else { w.join(" ") };
            out.push(phrase.to_lowercase());
        }
    }
    out
}

/// A line is "template-like" if it contains 3+ structured-bullet markers
/// like "Option A", "Question 1", "Step 2", "选项 A", "题目 1". These come
/// from multiple-choice / annotation prompts and pollute top-phrase stats.
fn is_template_line(line: &str) -> bool {
    static BULLET: Lazy<regex::Regex> = Lazy::new(|| {
        regex::Regex::new(
            r"(?i)\b(option|question|step|task|item|choice|answer|q|选项|题目|问题|步骤)\s*([a-z0-9一二三四五六七八九十]+)",
        )
        .unwrap()
    });
    BULLET.find_iter(line).count() >= 3
}

fn cjk_ratio(s: &str) -> f64 {
    let total = s.chars().count().max(1) as f64;
    let cjk = s.chars().filter(|c| {
        let cp = *c as u32;
        (0x4E00..=0x9FFF).contains(&cp) || (0x3000..=0x303F).contains(&cp)
    }).count() as f64;
    cjk / total
}

fn tokenize_en(s: &str) -> Vec<String> {
    static RE: Lazy<regex::Regex> = Lazy::new(|| regex::Regex::new(r"[a-zA-Z']+").unwrap());
    RE.find_iter(s).map(|m| m.as_str().to_lowercase()).collect()
}

fn tokenize_zh(s: &str) -> Vec<String> {
    static J: Lazy<jieba_rs::Jieba> = Lazy::new(jieba_rs::Jieba::new);
    J.cut(s, false)
        .into_iter()
        .filter(|t| {
            !t.trim().is_empty()
                && t.chars().any(|c| c.is_alphanumeric() || (c as u32) >= 0x4E00)
        })
        .map(|t| t.to_string())
        .collect()
}

const EN_STOPWORDS: &[&str] = &[
    "the","a","an","is","are","was","were","i","you","he","she","it","we","they",
    "and","or","but","if","of","to","in","on","for","with","at","by","from","as",
    "this","that","these","those","be","been","being","have","has","had","do","does","did",
    "will","would","can","could","should","may","might","must","shall",
    "my","your","our","their","his","her","its","me","us","them",
    "not","no","yes","ok","okay","please","just","so","then","than","very",
    "what","when","where","which","who","why","how",
    "am","im","ive","id","youre","were","theyre","wasnt","isnt","dont","didnt",
    "thats","theres","whats","heres",
    "about","into","over","under","again","also","only","more","most","some","any","all",
    "out","up","down","off","get","got","make","made","like","want","need",
];

const ZH_STOPWORDS: &[&str] = &[
    "的","了","和","是","就","都","而","及","与","或","一","个","在","有","也","上",
    "我","你","他","她","它","我们","你们","他们","这","那","什么","怎么","怎样",
    "吗","呢","吧","啊","哦","嗯","然后","但是","可是","就是","因为","所以",
    "的话","一下","一个","一些","一样","可以","能够","已经","还是","或者","并且",
    "把","让","给","对","到","用","从","向","被","比","跟","和","及","与",
    "啊","哈","嘛","呀","吖","哎","嗨","唉",
    "我的","你的","他的","她的","它的","我们的","你们的","他们的",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_top_phrases() {
        let texts = vec![
            "I don't understand the error".to_string(),
            "I don't understand why it fails".to_string(),
            "Can you explain the error".to_string(),
        ];
        let r = top_phrases(&texts, 5);
        assert!(r.iter().any(|p| p.phrase.contains("don't understand")), "got {:?}", r);
    }

    #[test]
    fn chinese_top_phrases() {
        let texts = vec![
            "我不明白这个错误".to_string(),
            "我不明白为什么".to_string(),
            "帮我解释一下错误".to_string(),
        ];
        let r = top_phrases(&texts, 5);
        assert!(r.iter().any(|p| p.phrase.contains("不明白")), "got {:?}", r);
    }

    #[test]
    fn singleton_phrases_filtered() {
        let texts = vec!["one off phrase here".to_string()];
        let r = top_phrases(&texts, 5);
        assert!(r.is_empty(), "single-occurrence phrases should be filtered");
    }

    #[test]
    fn template_phrases_are_demoted() {
        // Multiple-choice annotation template repeats "Question / Option" a lot.
        let mut texts: Vec<String> = (1..=20)
            .map(|i| format!("Question {i}. Option A: foo. Option B: bar. Option C: baz. Option D: qux."))
            .collect();
        // The user's actual catch-phrase appears far less often.
        for _ in 0..3 {
            texts.push("I don't understand this part".to_string());
            texts.push("I don't understand the result".to_string());
        }
        let r = top_phrases(&texts, 10);
        let phrases: Vec<&str> = r.iter().map(|p| p.phrase.as_str()).collect();
        let template_in_top3 = phrases
            .iter()
            .take(3)
            .any(|p| p.contains("option") || p.contains("question"));
        assert!(
            !template_in_top3,
            "template phrases should not dominate top 3, got {:?}",
            phrases
        );
        assert!(
            phrases.iter().any(|p| p.contains("don't understand")),
            "real catch-phrase should still appear, got {:?}",
            phrases
        );
    }

    #[test]
    fn template_filter_passes_normal_text() {
        // A line mentioning "option" once should NOT be flagged.
        assert!(!is_template_line("I'd choose option A here"));
        // 3+ Question/Option markers IS a template.
        assert!(is_template_line(
            "Question 1. Option A. Option B. Option C."
        ));
    }
}
