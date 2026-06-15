// Plugin system foundation for extensibility
// Provides a lightweight plugin API without complex dependency management

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Plugin metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub enabled: bool,
}

/// Plugin trait - base interface for all plugins
pub trait Plugin: Send + Sync {
    /// Get plugin metadata
    fn metadata(&self) -> PluginMetadata;

    /// Initialize the plugin
    fn init(&self) -> Result<(), String>;

    /// Cleanup when plugin is unloaded
    fn cleanup(&self) -> Result<(), String>;
}

/// Built-in plugin types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginType {
    /// File analysis plugin
    Analyzer,
    /// File operation plugin
    FileOperator,
    /// UI extension plugin
    UIExtension,
    /// Export plugin
    Exporter,
    /// Import plugin
    Importer,
}

/// Plugin descriptor with configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDescriptor {
    pub metadata: PluginMetadata,
    pub plugin_type: PluginType,
    pub config: Option<HashMap<String, String>>,
}

/// Plugin registry for managing available plugins
pub struct PluginRegistry {
    plugins: HashMap<String, Box<dyn Plugin>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        PluginRegistry {
            plugins: HashMap::new(),
        }
    }

    /// Register a plugin
    pub fn register(&mut self, plugin: Box<dyn Plugin>) -> Result<(), String> {
        let metadata = plugin.metadata();
        if self.plugins.contains_key(&metadata.id) {
            return Err(format!("Plugin '{}' is already registered", metadata.id));
        }

        plugin.init()?;
        self.plugins.insert(metadata.id.clone(), plugin);
        Ok(())
    }

    /// Unregister a plugin
    pub fn unregister(&mut self, plugin_id: &str) -> Result<(), String> {
        if let Some(plugin) = self.plugins.remove(plugin_id) {
            plugin.cleanup()?;
            Ok(())
        } else {
            Err(format!("Plugin '{}' not found", plugin_id))
        }
    }

    /// Get a plugin by ID
    pub fn get(&self, plugin_id: &str) -> Option<&dyn Plugin> {
        self.plugins.get(plugin_id).map(|p| p.as_ref() as &dyn Plugin)
    }

    /// List all registered plugins
    pub fn list(&self) -> Vec<PluginMetadata> {
        self.plugins.values()
            .map(|p| p.metadata())
            .collect()
    }

    /// Enable a plugin
    pub fn enable(&mut self, plugin_id: &str) -> Result<(), String> {
        if let Some(plugin) = self.plugins.get_mut(plugin_id) {
            // Re-initialize if was disabled
            plugin.init()?;
            Ok(())
        } else {
            Err(format!("Plugin '{}' not found", plugin_id))
        }
    }

    /// Disable a plugin
    pub fn disable(&mut self, plugin_id: &str) -> Result<(), String> {
        if let Some(plugin) = self.plugins.get_mut(plugin_id) {
            plugin.cleanup()?;
            Ok(())
        } else {
            Err(format!("Plugin '{}' not found", plugin_id))
        }
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// Built-in plugins

/// Example: File grouping analyzer plugin
pub struct FileGrouperPlugin;

impl Plugin for FileGrouperPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "builtin.file-grouper".to_string(),
            name: "File Grouper".to_string(),
            version: "1.0.0".to_string(),
            description: "Automatically groups related files together".to_string(),
            author: "小当家团队".to_string(),
            enabled: true,
        }
    }

    fn init(&self) -> Result<(), String> {
        log::info!("FileGrouperPlugin initialized");
        Ok(())
    }

    fn cleanup(&self) -> Result<(), String> {
        log::info!("FileGrouperPlugin cleaned up");
        Ok(())
    }
}

/// Example: Export to markdown plugin
pub struct MarkdownExporterPlugin;

impl Plugin for MarkdownExporterPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "builtin.markdown-exporter".to_string(),
            name: "Markdown Exporter".to_string(),
            version: "1.0.0".to_string(),
            description: "Export project structure to Markdown".to_string(),
            author: "小当家团队".to_string(),
            enabled: true,
        }
    }

    fn init(&self) -> Result<(), String> {
        log::info!("MarkdownExporterPlugin initialized");
        Ok(())
    }

    fn cleanup(&self) -> Result<(), String> {
        log::info!("MarkdownExporterPlugin cleaned up");
        Ok(())
    }
}

/// Create default plugin registry with built-in plugins
pub fn create_default_registry() -> PluginRegistry {
    let mut registry = PluginRegistry::new();

    // Register built-in plugins
    registry.register(Box::new(FileGrouperPlugin)).ok();
    registry.register(Box::new(MarkdownExporterPlugin)).ok();

    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestPlugin {
        id: String,
    }

    impl Plugin for TestPlugin {
        fn metadata(&self) -> PluginMetadata {
            PluginMetadata {
                id: self.id.clone(),
                name: "Test Plugin".to_string(),
                version: "1.0.0".to_string(),
                description: "A test plugin".to_string(),
                author: "Test".to_string(),
                enabled: true,
            }
        }

        fn init(&self) -> Result<(), String> {
            Ok(())
        }

        fn cleanup(&self) -> Result<(), String> {
            Ok(())
        }
    }

    #[test]
    fn test_register_plugin() {
        let mut registry = PluginRegistry::new();
        let result = registry.register(Box::new(TestPlugin { id: "test".to_string() }));
        assert!(result.is_ok());
        assert_eq!(registry.list().len(), 1);
    }

    #[test]
    fn test_duplicate_registration() {
        let mut registry = PluginRegistry::new();
        registry.register(Box::new(TestPlugin { id: "test".to_string() })).ok();
        let result = registry.register(Box::new(TestPlugin { id: "test".to_string() }));
        assert!(result.is_err());
    }

    #[test]
    fn test_unregister_plugin() {
        let mut registry = PluginRegistry::new();
        registry.register(Box::new(TestPlugin { id: "test".to_string() })).ok();
        let result = registry.unregister("test");
        assert!(result.is_ok());
        assert_eq!(registry.list().len(), 0);
    }

    #[test]
    fn test_get_plugin() {
        let mut registry = PluginRegistry::new();
        registry.register(Box::new(TestPlugin { id: "test".to_string() })).ok();
        let plugin = registry.get("test");
        assert!(plugin.is_some());
        assert_eq!(plugin.unwrap().metadata().id, "test");
    }

    #[test]
    fn test_default_registry() {
        let registry = create_default_registry();
        let plugins = registry.list();
        // Should have 2 built-in plugins
        assert!(plugins.len() >= 2);
    }
}