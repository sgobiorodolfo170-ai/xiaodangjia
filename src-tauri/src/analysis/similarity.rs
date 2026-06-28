use crate::db::FileNode;

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
