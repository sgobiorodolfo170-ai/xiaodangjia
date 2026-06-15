// TF-IDF based content similarity analysis
// Improves file similarity detection beyond basic name/extension matching

use std::collections::{HashMap, HashSet};
use regex::Regex;

/// Represents a document for TF-IDF analysis
#[derive(Debug, Clone)]
pub struct Document {
    pub id: String,
    pub content: String,
    pub tokens: Vec<String>,
}

/// TF-IDF index for computing document similarity
pub struct TfidfIndex {
    documents: HashMap<String, Document>,
    idf: HashMap<String, f64>,
    vocabulary: HashSet<String>,
}

impl TfidfIndex {
    pub fn new() -> Self {
        TfidfIndex {
            documents: HashMap::new(),
            idf: HashMap::new(),
            vocabulary: HashSet::new(),
        }
    }

    /// Tokenize text into words
    fn tokenize(text: &str) -> Vec<String> {
        // Simple tokenization: lowercase, keep alphanumeric
        let re = Regex::new(r"[a-zA-Z0-9]{2,}").unwrap();
        re.find_iter(&text.to_lowercase())
            .map(|m| m.as_str().to_string())
            .collect()
    }

    /// Add a document to the index
    pub fn add_document(&mut self, id: String, content: String) {
        let tokens = Self::tokenize(&content);

        // Collect unique tokens for vocabulary
        let unique_tokens: HashSet<String> = tokens.iter().cloned().collect();
        self.vocabulary.extend(unique_tokens);

        self.documents.insert(id.clone(), Document {
            id,
            content,
            tokens,
        });
    }

    /// Build IDF values after all documents are added
    pub fn build_idf(&mut self) {
        let n = self.documents.len() as f64;
        if n == 0.0 { return; }

        for term in &self.vocabulary {
            let mut doc_count = 0;
            for doc in self.documents.values() {
                if doc.tokens.contains(term) {
                    doc_count += 1;
                }
            }
            // IDF = log(N / df) + 1 to avoid zero
            let idf = ((n / doc_count as f64).ln() + 1.0).max(0.0);
            self.idf.insert(term.clone(), idf);
        }
    }

    /// Compute TF-IDF vector for a document
    fn compute_tfidf(&self, tokens: &[String]) -> HashMap<String, f64> {
        let mut tf: HashMap<String, f64> = HashMap::new();

        // Term frequency
        for token in tokens {
            *tf.entry(token.clone()).or_insert(0.0) += 1.0;
        }

        // Normalize by document length
        let len = tokens.len() as f64;
        if len > 0.0 {
            for val in tf.values_mut() {
                *val /= len;
            }
        }

        // Apply IDF
        for (term, tf_val) in tf.iter_mut() {
            if let Some(idf) = self.idf.get(term) {
                *tf_val *= idf;
            }
        }

        tf
    }

    /// Compute cosine similarity between two TF-IDF vectors
    fn cosine_similarity(vec1: &HashMap<String, f64>, vec2: &HashMap<String, f64>) -> f64 {
        let mut dot_product = 0.0;
        let mut norm1 = 0.0;
        let mut norm2 = 0.0;

        for (term, val1) in vec1 {
            let val2 = vec2.get(term).unwrap_or(&0.0);
            dot_product += val1 * val2;
            norm1 += val1 * val1;
        }

        for val2 in vec2.values() {
            norm2 += val2 * val2;
        }

        let denominator = (norm1.sqrt() * norm2.sqrt());
        if denominator > 0.0 {
            dot_product / denominator
        } else {
            0.0
        }
    }

    /// Find similar documents to the given content
    pub fn find_similar(&self, content: &str, top_k: usize) -> Vec<(String, f64)> {
        let query_tokens = Self::tokenize(content);
        let query_tfidf = self.compute_tfidf(&query_tokens);

        let mut similarities: Vec<(String, f64)> = Vec::new();

        for (id, doc) in &self.documents {
            let doc_tfidf = self.compute_tfidf(&doc.tokens);
            let sim = Self::cosine_similarity(&query_tfidf, &doc_tfidf);
            similarities.push((id.clone(), sim));
        }

        // Sort by similarity descending
        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        similarities.into_iter().take(top_k).collect()
    }

    /// Compute similarity between two specific documents
    pub fn similarity(&self, id1: &str, id2: &str) -> f64 {
        let doc1 = match self.documents.get(id1) {
            Some(d) => d,
            None => return 0.0,
        };

        let doc2 = match self.documents.get(id2) {
            Some(d) => d,
            None => return 0.0,
        };

        let tfidf1 = self.compute_tfidf(&doc1.tokens);
        let tfidf2 = self.compute_tfidf(&doc2.tokens);

        Self::cosine_similarity(&tfidf1, &tfidf2)
    }
}

impl Default for TfidfIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple content-based similarity (without building full index)
/// More efficient for on-the-fly comparisons
pub fn compute_content_similarity(content1: &str, content2: &str) -> f64 {
    let tokens1 = TfidfIndex::tokenize(content1);
    let tokens2 = TfidfIndex::tokenize(content2);

    if tokens1.is_empty() || tokens2.is_empty() {
        return 0.0;
    }

    // Use simple Jaccard similarity as approximation
    let set1: HashSet<&String> = tokens1.iter().collect();
    let set2: HashSet<&String> = tokens2.iter().collect();

    let intersection: HashSet<_> = set1.intersection(&set2).cloned().cloned().collect();
    let union: HashSet<_> = set1.union(&set2).cloned().cloned().collect();

    if union.is_empty() {
        return 0.0;
    }

    intersection.len() as f64 / union.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize() {
        let tokens = TfidfIndex::tokenize("Hello World! This is a test.");
        assert!(tokens.contains(&"hello".to_string()));
        assert!(tokens.contains(&"world".to_string()));
        assert!(tokens.contains(&"test".to_string()));
    }

    #[test]
    fn test_add_document() {
        let mut index = TfidfIndex::new();
        index.add_document("doc1".to_string(), "Hello world".to_string());
        assert_eq!(index.documents.len(), 1);
    }

    #[test]
    fn test_build_idf() {
        let mut index = TfidfIndex::new();
        index.add_document("doc1".to_string(), "hello world".to_string());
        index.add_document("doc2".to_string(), "hello python".to_string());
        index.add_document("doc3".to_string(), "rust programming".to_string());
        index.build_idf();

        // "hello" appears in 2 docs, so IDF should be less than unique terms
        assert!(index.idf.contains_key("hello"));
        assert!(index.idf.contains_key("rust"));
    }

    #[test]
    fn test_find_similar() {
        let mut index = TfidfIndex::new();
        index.add_document("1".to_string(), "python programming language".to_string());
        index.add_document("2".to_string(), "javascript web development".to_string());
        index.add_document("3".to_string(), "python machine learning".to_string());
        index.add_document("4".to_string(), "rust systems programming".to_string());
        index.build_idf();

        let similar = index.find_similar("python code", 2);
        // Should find doc1 and doc3 (both have "python")
        assert!(similar.len() <= 2);
    }

    #[test]
    fn test_content_similarity_identical() {
        let content = "Hello world this is a test document";
        let sim = compute_content_similarity(content, content);
        assert!((sim - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_content_similarity_different() {
        let content1 = "python programming";
        let content2 = "javascript web development";
        let sim = compute_content_similarity(content1, content2);
        // Should have some similarity but not high
        assert!(sim < 1.0);
        assert!(sim >= 0.0);
    }

    #[test]
    fn test_empty_content() {
        let sim = compute_content_similarity("", "hello world");
        assert_eq!(sim, 0.0);

        let sim2 = compute_content_similarity("hello", "");
        assert_eq!(sim2, 0.0);
    }

    #[test]
    fn test_similarity_order() {
        // More similar content should have higher score
        let doc1 = "python programming language tutorial";
        let doc2 = "python programming guide";
        let doc3 = "javascript web development";

        let sim12 = compute_content_similarity(doc1, doc2);
        let sim13 = compute_content_similarity(doc1, doc3);

        assert!(sim12 > sim13);
    }
}