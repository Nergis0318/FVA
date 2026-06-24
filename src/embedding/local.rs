//! Local hash-based embeddings (no API required).
//!
//! Uses feature hashing (similar to HashingVectorizer) for fast,
//! deterministic embeddings suitable for code semantic search.

use super::{Embedder, normalize};
use crate::error::Result;

pub struct LocalEmbedder {
    dimensions: usize,
}

impl LocalEmbedder {
    pub fn new(dimensions: usize) -> Self {
        Self {
            dimensions: dimensions.max(64),
        }
    }

    fn hash_embed(&self, text: &str) -> Vec<f32> {
        let dims = self.dimensions;
        let mut vec = vec![0.0f32; dims];
        let lower = text.to_lowercase();

        // Word-level features (FNV-1a hash instead of blake3)
        for token in lower.split(|c: char| !c.is_alphanumeric() && c != '_') {
            if token.len() < 2 {
                continue;
            }
            add_feature(&mut vec, token.as_bytes(), 1.0, dims);
        }

        // Character trigrams for typo tolerance — hash bytes directly,
        // avoiding String allocation per trigram.
        let bytes = lower.as_bytes();
        if bytes.len() >= 3 {
            for w in bytes.windows(3) {
                let h = hash_bytes(w);
                let idx = (h as usize) % dims;
                let sign = if h & 1 == 0 { 1.0 } else { -1.0 };
                vec[idx] += sign * 0.5;
            }
        }

        // Identifier tokens (CamelCase / snake_case splits)
        for token in text.split(|c: char| !c.is_alphanumeric()) {
            if token.is_empty() {
                continue;
            }
            for_each_identifier_part(token, |part| {
                add_feature(&mut vec, fast_lower(part).as_bytes(), 0.8, dims);
            });
        }

        normalize(&mut vec);
        vec
    }
}

/// FNV-1a 64-bit hash for a byte slice.
fn hash_bytes(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325; // FNV-1a offset basis
    for &b in data {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100_0000_01b3); // FNV-1a prime
    }
    hash
}

/// Hash a feature and add its weighted contribution to the vector.
fn add_feature(vec: &mut [f32], feature: &[u8], weight: f32, dims: usize) {
    let h = hash_bytes(feature);
    let idx = (h as usize) % dims;
    let sign = if h & 1 == 0 { 1.0 } else { -1.0 };
    vec[idx] += sign * weight;
}

/// Fast lowercase of a short ASCII-biased string (allocates only for non-ASCII).
fn fast_lower(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        out.push(ch.to_ascii_lowercase());
    }
    out
}

/// Walk identifier parts without allocating a Vec<String>.
fn for_each_identifier_part<F>(s: &str, mut f: F)
where
    F: FnMut(&str),
{
    let mut start = 0usize;
    let chars: Vec<char> = s.chars().collect();
    let mut prev_lower = false;

    for (i, &ch) in chars.iter().enumerate() {
        let is_upper = ch.is_uppercase();
        let is_lower = ch.is_lowercase();

        if is_upper && prev_lower && i > start {
            f(&s[start..i]);
            start = i;
        }
        if ch == '_' {
            if i > start {
                f(&s[start..i]);
            }
            start = i + 1;
            prev_lower = false;
            continue;
        }
        prev_lower = is_lower;
    }
    if start < chars.len() {
        f(&s[start..]);
    }
    if chars.is_empty() {
        f(s);
    }
}

impl Embedder for LocalEmbedder {
    fn name(&self) -> &str {
        "local-hash"
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }

    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        Ok(texts.iter().map(|t| self.hash_embed(t)).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embed_produces_normalized_vector() {
        let e = LocalEmbedder::new(128);
        let v = e
            .embed_one("fn hello_world() { println!(\"hi\"); }")
            .unwrap();
        assert_eq!(v.len(), 128);
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.01);
    }

    #[test]
    fn similar_code_has_higher_similarity() {
        let e = LocalEmbedder::new(256);
        let a = e
            .embed_one("fn authenticate_user(token: &str) -> Result<User>")
            .unwrap();
        let b = e
            .embed_one("fn authenticate_user(session: &str) -> Result<User>")
            .unwrap();
        let c = e
            .embed_one("fn render_html_template(page: &str) -> String")
            .unwrap();
        let sim_ab = super::super::cosine_similarity(&a, &b);
        let sim_ac = super::super::cosine_similarity(&a, &c);
        assert!(sim_ab > sim_ac);
    }
}
