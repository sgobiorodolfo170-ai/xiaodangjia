use crate::db::FileNode;
use crate::semantic_embedding::SemanticPatterns;
use crate::tfidf::TfidfIndex;
use std::fs;

/// A scored node returned by similarity searches.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoredNode {
    pub node_id: String,
    pub score: f64,
}

/// Unified trait for file similarity strategies (P2-5).
/// All similarity engines implement this interface so callers
/// can swap strategies without changing downstream code.
pub trait FileSimilarity: Send + Sync {
    /// Find files similar to `target` among `candidates`.
    fn find_similar(&self, target: &FileNode, candidates: &[FileNode]) -> Vec<ScoredNode>;
}

/// TF-IDF engine: content-based similarity using term frequency analysis.
pub struct TfidfSimilarity;

impl FileSimilarity for TfidfSimilarity {
    fn find_similar(&self, target: &FileNode, candidates: &[FileNode]) -> Vec<ScoredNode> {
        let target_content = read_content(&target.path);
        if target_content.is_empty() {
            return Vec::new();
        }

        let mut index = TfidfIndex::new();
        for node in candidates {
            if node.id == target.id || node.is_directory { continue; }
            if node.size > 1_000_000 { continue; }
            let content = read_content(&node.path);
            if !content.is_empty() {
                index.add_document(node.id.clone(), content);
            }
        }
        index.build_idf();

        index.find_similar(&target_content, candidates.len())
            .into_iter()
            .map(|(id, score)| ScoredNode { node_id: id, score })
            .collect()
    }
}

/// Embedding engine: semantic similarity using pattern-based embeddings.
pub struct EmbeddingSimilarity;

impl FileSimilarity for EmbeddingSimilarity {
    fn find_similar(&self, target: &FileNode, candidates: &[FileNode]) -> Vec<ScoredNode> {
        let patterns = SemanticPatterns::new();
        let target_content = read_content(&target.path);
        if target_content.is_empty() {
            return Vec::new();
        }
        let target_emb = patterns.generate_embedding(&target.name, &target.path, &target.extension, &target_content);

        let mut emb_candidates: Vec<(String, Vec<f64>)> = Vec::new();
        for node in candidates {
            if node.id == target.id || node.is_directory { continue; }
            let content = read_content(&node.path);
            let emb = patterns.generate_embedding(&node.name, &node.path, &node.extension, &content);
            emb_candidates.push((node.id.clone(), emb));
        }

        patterns.find_similar_by_embedding(&target_emb, &emb_candidates, candidates.len())
            .into_iter()
            .map(|(id, score)| ScoredNode { node_id: id, score })
            .collect()
    }
}

/// Read file content from disk, returning empty string on error.
fn read_content(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_default()
}
