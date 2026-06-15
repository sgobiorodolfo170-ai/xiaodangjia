// Semantic search module with keyword expansion and synonym matching
// Provides natural language search capability without heavy vector database

use std::collections::HashMap;
use regex::Regex;

/// Synonym groups for common programming terms
/// This provides basic semantic understanding without embeddings
pub struct SemanticSearch {
    synonyms: HashMap<String, Vec<String>>,
}

impl SemanticSearch {
    pub fn new() -> Self {
        let mut synonyms = HashMap::new();

        // Programming language synonyms
        synonyms.insert("code".to_string(), vec!["script".to_string(), "program".to_string()]);
        synonyms.insert("script".to_string(), vec!["code".to_string(), "program".to_string()]);
        synonyms.insert("program".to_string(), vec!["code".to_string(), "script".to_string()]);

        // File type synonyms
        synonyms.insert("style".to_string(), vec!["css".to_string(), "styling".to_string(), "theme".to_string()]);
        synonyms.insert("css".to_string(), vec!["style".to_string(), "stylesheet".to_string()]);
        synonyms.insert("config".to_string(), vec!["configuration".to_string(), "settings".to_string()]);
        synonyms.insert("test".to_string(), vec!["testing".to_string(), "spec".to_string(), "specs".to_string()]);

        // Framework synonyms
        synonyms.insert("frontend".to_string(), vec!["ui".to_string(), "web".to_string(), "client".to_string()]);
        synonyms.insert("backend".to_string(), vec!["server".to_string(), "api".to_string(), "service".to_string()]);
        synonyms.insert("api".to_string(), vec!["endpoint".to_string(), "service".to_string(), "backend".to_string()]);
        synonyms.insert("db".to_string(), vec!["database".to_string(), "storage".to_string(), "data".to_string()]);
        synonyms.insert("database".to_string(), vec!["db".to_string(), "storage".to_string(), "data".to_string()]);

        // Common project structure
        synonyms.insert("src".to_string(), vec!["source".to_string(), "lib".to_string(), "code".to_string()]);
        synonyms.insert("docs".to_string(), vec!["documentation".to_string(), "doc".to_string(), "readme".to_string()]);
        synonyms.insert("build".to_string(), vec!["dist".to_string(), "output".to_string(), "compile".to_string()]);
        synonyms.insert("util".to_string(), vec!["utility".to_string(), "helpers".to_string(), "common".to_string()]);
        synonyms.insert("helper".to_string(), vec!["util".to_string(), "utility".to_string(), "common".to_string()]);
        synonyms.insert("model".to_string(), vec!["models".to_string(), "schema".to_string(), "entity".to_string()]);
        synonyms.insert("view".to_string(), vec!["views".to_string(), "page".to_string(), "screen".to_string()]);
        synonyms.insert("controller".to_string(), vec!["controllers".to_string(), "handler".to_string(), "endpoint".to_string()]);
        synonyms.insert("component".to_string(), vec!["components".to_string(), "widget".to_string(), "element".to_string()]);

        // Language-specific
        synonyms.insert("js".to_string(), vec!["javascript".to_string(), "ecmascript".to_string()]);
        synonyms.insert("ts".to_string(), vec!["typescript".to_string()]);
        synonyms.insert("py".to_string(), vec!["python".to_string()]);
        synonyms.insert("rs".to_string(), vec!["rust".to_string()]);
        synonyms.insert("go".to_string(), vec!["golang".to_string()]);

        SemanticSearch { synonyms }
    }

    /// Expand query with synonyms
    pub fn expand_query(&self, query: &str) -> Vec<String> {
        let mut expanded = Vec::new();
        let mut seen = std::collections::HashSet::new();

        let words: Vec<String> = query
            .to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        for word in &words {
            if seen.insert(word.clone()) {
                expanded.push(word.clone());
            }

            // Add direct synonyms
            if let Some(syns) = self.synonyms.get(word) {
                for syn in syns {
                    if seen.insert(syn.clone()) {
                        expanded.push(syn.clone());
                    }
                }
            }

            // Also check if any synonym maps TO this word
            for (key, syns) in &self.synonyms {
                if syns.contains(word) && seen.insert(key.clone()) {
                    expanded.push(key.clone());
                }
            }
        }

        expanded
    }

    /// Search with semantic expansion
    pub fn search(&self, items: &[SearchableItem], query: &str) -> Vec<SearchResult> {
        let expanded = self.expand_query(query);
        if expanded.is_empty() {
            return Vec::new();
        }

        let mut results: Vec<SearchResult> = Vec::new();

        for item in items {
            let mut score = 0.0;
            let query_lower = query.to_lowercase();

            // Direct match (highest score)
            if item.name.to_lowercase().contains(&query_lower) {
                score += 1.0;
            }

            // Expanded term matches
            for term in &expanded {
                let term_lower = term.to_lowercase();

                // Name match
                if item.name.to_lowercase().contains(&term_lower) {
                    score += 0.8;
                }

                // Path match
                if item.path.to_lowercase().contains(&term_lower) {
                    score += 0.5;
                }

                // Tag match (high value)
                for tag in &item.tags {
                    if tag.to_lowercase().contains(&term_lower) {
                        score += 0.7;
                    }
                }

                // Extension match
                if item.extension.to_lowercase() == term_lower {
                    score += 0.6;
                }
            }

            // Bonus for exact match
            if item.name.to_lowercase() == query_lower {
                score += 0.5;
            }

            if score > 0.0 {
                results.push(SearchResult {
                    item: item.clone(),
                    score,
                    matched_terms: expanded.clone(),
                });
            }
        }

        // Sort by score descending
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        results
    }
}

impl Default for SemanticSearch {
    fn default() -> Self {
        Self::new()
    }
}

/// Item that can be searched
#[derive(Debug, Clone)]
pub struct SearchableItem {
    pub id: String,
    pub name: String,
    pub path: String,
    pub extension: String,
    pub tags: Vec<String>,
}

/// Search result with score
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub item: SearchableItem,
    pub score: f64,
    pub matched_terms: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item(id: &str, name: &str, path: &str, ext: &str, tags: Vec<&str>) -> SearchableItem {
        SearchableItem {
            id: id.to_string(),
            name: name.to_string(),
            path: path.to_string(),
            extension: ext.to_string(),
            tags: tags.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn test_expand_query_simple() {
        let search = SemanticSearch::new();
        let expanded = search.expand_query("css");
        assert!(expanded.contains(&"css".to_string()));
        assert!(expanded.contains(&"style".to_string()));
    }

    #[test]
    fn test_expand_query_no_match() {
        let search = SemanticSearch::new();
        let expanded = search.expand_query("unknownword");
        assert!(expanded.contains(&"unknownword".to_string()));
    }

    #[test]
    fn test_search_direct_match() {
        let search = SemanticSearch::new();
        let items = vec![
            make_item("1", "app.ts", "/src/app.ts", "ts", vec!["TypeScript"]),
            make_item("2", "styles.css", "/src/styles.css", "css", vec!["Styles"]),
        ];

        let results = search.search(&items, "app.ts");
        assert!(!results.is_empty());
        assert_eq!(results[0].item.name, "app.ts");
    }

    #[test]
    fn test_search_synonym_match() {
        let search = SemanticSearch::new();
        let items = vec![
            make_item("1", "main.css", "/src/main.css", "css", vec!["Styles"]),
            make_item("2", "app.ts", "/src/app.ts", "ts", vec!["TypeScript"]),
        ];

        // Search for "style" should match "css" files
        let results = search.search(&items, "style");
        assert!(!results.is_empty());
        // CSS file should have higher score due to synonym match
        assert_eq!(results[0].item.extension, "css");
    }

    #[test]
    fn test_search_tag_match() {
        let search = SemanticSearch::new();
        let items = vec![
            make_item("1", "test.spec.ts", "/test.spec.ts", "ts", vec!["测试"]),
            make_item("2", "app.ts", "/app.ts", "ts", vec!["TypeScript"]),
        ];

        // Search for "testing" should match "test" file via synonym
        let results = search.search(&items, "test");
        assert!(!results.is_empty());
    }

    #[test]
    fn test_search_empty_query() {
        let search = SemanticSearch::new();
        let items = vec![
            make_item("1", "app.ts", "/app.ts", "ts", vec![]),
        ];

        let results = search.search(&items, "");
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_results_sorted() {
        let search = SemanticSearch::new();
        let items = vec![
            make_item("1", "app.ts", "/src/app.ts", "ts", vec!["TypeScript"]),
            make_item("2", "application.ts", "/src/application.ts", "ts", vec!["TypeScript"]),
        ];

        let results = search.search(&items, "app");
        assert!(results.len() >= 2);
        // Exact match should be first
        assert_eq!(results[0].item.name, "app.ts");
    }

    #[test]
    fn test_search_api_synonyms() {
        let search = SemanticSearch::new();
        let items = vec![
            make_item("1", "api.ts", "/src/api.ts", "ts", vec!["API"]),
            make_item("2", "server.py", "/src/server.py", "py", vec!["Backend"]),
        ];

        let results = search.search(&items, "backend");
        assert!(!results.is_empty());
    }
}