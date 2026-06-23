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
        let mut vec = vec![0.0f32; self.dimensions];
        let lower = text.to_lowercase();

        // Word-level features
        for token in lower.split(|c: char| !c.is_alphanumeric() && c != '_') {
            if token.len() < 2 {
                continue;
            }
            self.add_feature(&mut vec, token, 1.0);
        }

        // Character trigrams for typo tolerance
        let chars: Vec<char> = lower.chars().collect();
        for window in chars.windows(3) {
            let tri: String = window.iter().collect();
            self.add_feature(&mut vec, &tri, 0.5);
        }

        // Identifier tokens (CamelCase / snake_case splits)
        for token in text.split(|c: char| !c.is_alphanumeric()) {
            if token.is_empty() {
                continue;
            }
            for part in split_identifier(token) {
                self.add_feature(&mut vec, &part.to_lowercase(), 0.8);
            }
        }

        normalize(&mut vec);
        vec
    }

    fn add_feature(&self, vec: &mut [f32], feature: &str, weight: f32) {
        let hash = blake3::hash(feature.as_bytes());
        let bytes = hash.as_bytes();
        let h = u64::from_le_bytes(bytes[..8].try_into().unwrap());
        let idx = (h as usize) % self.dimensions;
        let sign = if h & 1 == 0 { 1.0 } else { -1.0 };
        vec[idx] += sign * weight;
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

fn split_identifier(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut prev_lower = false;

    for ch in s.chars() {
        let is_upper = ch.is_uppercase();
        let is_lower = ch.is_lowercase();

        if is_upper && prev_lower && !current.is_empty() {
            parts.push(current.clone());
            current.clear();
        }
        if ch == '_' {
            if !current.is_empty() {
                parts.push(current.clone());
                current.clear();
            }
            prev_lower = false;
            continue;
        }
        current.push(ch);
        prev_lower = is_lower;
    }
    if !current.is_empty() {
        parts.push(current);
    }
    if parts.is_empty() {
        parts.push(s.to_string());
    }
    parts
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
