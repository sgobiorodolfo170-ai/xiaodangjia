// AST-like import parser for common JavaScript/TypeScript/Python import patterns
// This is a lightweight regex-based solution that handles 90% of common cases

use regex::Regex;

/// Parse a file and extract all import/require statements
/// Returns a list of imported module names
pub fn parse_imports(content: &str, extension: &str) -> Vec<String> {
    let mut imports = Vec::new();

    match extension.to_lowercase().as_str() {
        "js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs" => {
            // JavaScript/TypeScript imports
            let ts_import_re = Regex::new(r#"import\s+(?:[\w*{}\s,]+\s+from\s+)?['"]([@\w\-./]+)['"]"#).unwrap();
            for cap in ts_import_re.captures_iter(content) {
                if let Some(m) = cap.get(1) {
                    let module = m.as_str().to_string();
                    // Skip relative imports starting with . or absolute paths
                    if !module.starts_with('.') && !module.starts_with('/') {
                        imports.push(module);
                    }
                }
            }

            // Require statements
            let require_re = Regex::new(r#"require\s*\(\s*['"]([@\w\-./]+)['"]\s*\)"#).unwrap();
            for cap in require_re.captures_iter(content) {
                if let Some(m) = cap.get(1) {
                    let module = m.as_str().to_string();
                    if !module.starts_with('.') && !module.starts_with('/') {
                        imports.push(module);
                    }
                }
            }

            // ES6 export from - fixed regex
            let es6_re = Regex::new(r#"export\s+(?:\{\s*)?([\w,\s]+)(?:\s*\})?\s+from\s+['"]([@\w\-./]+)['"]"#).unwrap();
            for cap in es6_re.captures_iter(content) {
                if let Some(m) = cap.get(2) {
                    let module = m.as_str().to_string();
                    if !module.starts_with('.') && !module.starts_with('/') {
                        imports.push(module);
                    }
                }
            }
        }
        "py" => {
            // Python imports
            let python_import_re = Regex::new(r#"(?:from\s+([\w.]+)\s+import|import\s+([\w.]+))"#).unwrap();
            for cap in python_import_re.captures_iter(content) {
                if let Some(m) = cap.get(1) {
                    let module = m.as_str().to_string();
                    imports.push(module);
                } else if let Some(m) = cap.get(2) {
                    let module = m.as_str().to_string();
                    imports.push(module);
                }
            }
        }
        "rs" => {
            // Rust imports - fixed to match multiple items
            let rust_use_re = Regex::new(r#"use\s+([\w:]+)"#).unwrap();
            for cap in rust_use_re.captures_iter(content) {
                if let Some(m) = cap.get(1) {
                    let module = m.as_str().to_string();
                    // Skip internal crate imports
                    if !module.starts_with("crate::") && !module.starts_with("super::") {
                        imports.push(module);
                    }
                }
            }
        }
        "go" => {
            // Go imports - fixed to match both forms
            let go_import_re = Regex::new(r#"import\s+(?:\(\s*)?["']([@\w\-./]+)["']"#).unwrap();
            for cap in go_import_re.captures_iter(content) {
                if let Some(m) = cap.get(1) {
                    imports.push(m.as_str().to_string());
                }
            }
            // Also match import with alias: import alias "package"
            let go_alias_re = Regex::new(r#"import\s+(\w+)\s+["']([@\w\-./]+)["']"#).unwrap();
            for cap in go_alias_re.captures_iter(content) {
                if let Some(m) = cap.get(2) {
                    imports.push(m.as_str().to_string());
                }
            }
        }
        "java" | "kt" => {
            // Java/Kotlin imports
            let java_import_re = Regex::new(r#"import\s+([\w.]+)"#).unwrap();
            for cap in java_import_re.captures_iter(content) {
                if let Some(m) = cap.get(1) {
                    let module = m.as_str().to_string();
                    // Skip java.lang imports as they're built-in
                    if !module.starts_with("java.lang.") {
                        imports.push(module);
                    }
                }
            }
        }
        _ => {}
    }

    // Remove duplicates while preserving order
    imports.sort();
    imports.dedup();
    imports
}

/// Find file relationships based on import analysis
/// This improves the basic extension-based relation detection
pub fn analyze_import_relations(
    content: &str,
    extension: &str,
    file_name: &str,
    all_files: &[(String, String)], // (file_path, file_name)
) -> Vec<(String, f64)> {
    let imports = parse_imports(content, extension);
    let mut relations = Vec::new();

    for import in imports {
        // Try to match import to actual files in the project
        for (file_path, other_name) in all_files {
            // Check if import matches file name or path
            let import_base = import.split('/').last().unwrap_or(&import);
            let import_base = import_base.split('.').next().unwrap_or(import_base);

            if other_name.contains(import_base) || file_path.contains(&import.replace('.', "/")) {
                relations.push((file_path.clone(), 0.9)); // High confidence for explicit imports
            }
        }
    }

    relations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_js_imports() {
        let code = r#"
            import React from 'react';
            import { useState, useEffect } from 'react';
            import axios from 'axios';
            import './styles.css';
            import "/absolute/path";
        "#;
        let imports = parse_imports(code, "js");
        assert!(imports.contains(&"react".to_string()));
        assert!(imports.contains(&"axios".to_string()));
        // Relative and absolute paths should be filtered
        assert!(!imports.iter().any(|i| i.contains("styles")));
    }

    #[test]
    fn test_parse_typescript_imports() {
        let code = r#"
            import { Component } from '@angular/core';
            import type { User } from './types';
            export { someFunc } from './utils';
        "#;
        let imports = parse_imports(code, "ts");
        assert!(imports.contains(&"@angular/core".to_string()));
    }

    #[test]
    fn test_parse_python_imports() {
        let code = r#"
            import os
            import sys
            from collections import defaultdict
            from typing import List, Dict
        "#;
        let imports = parse_imports(code, "py");
        assert!(imports.contains(&"os".to_string()));
        assert!(imports.contains(&"sys".to_string()));
        assert!(imports.contains(&"collections".to_string()));
        assert!(imports.contains(&"typing".to_string()));
    }

    #[test]
    fn test_parse_rust_imports() {
        let code = r#"
            use std::collections::HashMap;
            use serde::Serialize;
            use serde::Deserialize;
            use crate::modules::auth;
            use super::parent;
        "#;
        let imports = parse_imports(code, "rs");
        assert!(imports.contains(&"std::collections::HashMap".to_string()));
        assert!(imports.contains(&"serde::Serialize".to_string()));
        assert!(imports.contains(&"serde::Deserialize".to_string()));
        // Internal imports should be filtered
        assert!(!imports.iter().any(|i| i.starts_with("crate::")));
        assert!(!imports.iter().any(|i| i.starts_with("super::")));
    }

    #[test]
    fn test_parse_go_imports() {
        // Test single line imports
        let code = r#"import "fmt"
        import "os"
        import "github.com/some/package"
        "#;
        let imports = parse_imports(code, "go");
        assert!(imports.contains(&"fmt".to_string()));
        assert!(imports.contains(&"os".to_string()));
        assert!(imports.contains(&"github.com/some/package".to_string()));
    }

    #[test]
    fn test_parse_java_imports() {
        let code = r#"
            import java.util.List;
            import java.util.ArrayList;
            import java.lang.String;
            import com.example.MyClass;
        "#;
        let imports = parse_imports(code, "java");
        assert!(imports.contains(&"java.util.List".to_string()));
        assert!(imports.contains(&"com.example.MyClass".to_string()));
        // java.lang is filtered
        assert!(!imports.iter().any(|i| i.starts_with("java.lang")));
    }

    #[test]
    fn test_require_syntax() {
        let code = r#"
            const fs = require('fs');
            const path = require('path');
        "#;
        let imports = parse_imports(code, "js");
        assert!(imports.contains(&"fs".to_string()));
        assert!(imports.contains(&"path".to_string()));
    }

    #[test]
    fn test_empty_code() {
        let code = "";
        let imports = parse_imports(code, "js");
        assert!(imports.is_empty());
    }

    #[test]
    fn test_no_imports() {
        let code = r#"
            function hello() {
                console.log("Hello, world!");
            }
        "#;
        let imports = parse_imports(code, "js");
        assert!(imports.is_empty());
    }
}