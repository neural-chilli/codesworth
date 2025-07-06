use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::collections::HashMap;

use crate::error::{CodesworthError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Whether to use LLM enhancement
    pub enabled: bool,

    /// LLM provider (rig-openai, rig-anthropic, rig-google, rig-ollama, cortex-gemini, cortex-claude)
    pub provider: String,

    /// Model name (e.g., "gpt-4", "claude-3-sonnet", "gemini-pro")
    pub model: String,

    /// API key (for external providers)
    pub api_key: Option<String>,

    /// Base URL (for Cortex API or custom endpoints)
    pub base_url: Option<String>,

    /// Maximum tokens for LLM responses
    pub max_tokens: Option<u32>,

    /// Temperature for LLM responses (0.0 to 1.0)
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Project configuration
    pub project: ProjectConfig,

    /// Source code parsing configuration
    pub parsing: ParsingConfig,

    /// Documentation generation settings
    pub generation: GenerationConfig,

    /// Template customization
    pub templates: TemplateConfig,

    /// Output settings
    pub output: OutputConfig,

    /// LLM integration settings
    pub llm: LlmConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Project name
    pub name: String,

    /// Source directories to analyze
    pub source_dirs: Vec<PathBuf>,

    /// Directories to ignore
    pub ignore_patterns: Vec<String>,

    /// Documentation output directory
    pub docs_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsingConfig {
    /// Languages to support
    pub languages: Vec<String>,

    /// File extensions to parse
    pub file_extensions: HashMap<String, String>,

    /// Maximum file size to parse (in bytes)
    pub max_file_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationConfig {
    /// Whether to generate API documentation
    pub generate_api_docs: bool,

    /// Whether to generate architecture overviews
    pub generate_architecture: bool,

    /// Whether to preserve existing human edits
    pub preserve_edits: bool,

    /// Content hash algorithm
    pub hash_algorithm: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateConfig {
    /// Custom template directory
    pub template_dir: Option<PathBuf>,

    /// Template settings
    pub settings: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    /// Default output format
    pub format: String,

    /// Include metadata headers
    pub include_metadata: bool,

    /// Static site generator compatibility
    pub static_site_compatible: bool,
}

impl Default for Config {
    fn default() -> Self {
        let mut file_extensions = HashMap::new();
        file_extensions.insert("rust".to_string(), "rs".to_string());
        file_extensions.insert("java".to_string(), "java".to_string());
        file_extensions.insert("python".to_string(), "py".to_string());
        file_extensions.insert("csharp".to_string(), "cs".to_string());
        file_extensions.insert("javascript".to_string(), "js".to_string());
        file_extensions.insert("typescript".to_string(), "ts".to_string());

        let mut template_settings = HashMap::new();
        template_settings.insert("author".to_string(), "Unknown".to_string());
        template_settings.insert("organization".to_string(), "".to_string());

        Self {
            project: ProjectConfig {
                name: "Unnamed Project".to_string(),
                source_dirs: vec![PathBuf::from("src")],
                ignore_patterns: vec![
                    "target/".to_string(),
                    "node_modules/".to_string(),
                    ".git/".to_string(),
                    "*.tmp".to_string(),
                ],
                docs_dir: PathBuf::from("docs"),
            },
            parsing: ParsingConfig {
                languages: vec!["rust".to_string(), "java".to_string(), "python".to_string(), "csharp".to_string(), "javascript".to_string()],
                file_extensions,
                max_file_size: 1024 * 1024, // 1MB
            },
            generation: GenerationConfig {
                generate_api_docs: true,
                generate_architecture: true,
                preserve_edits: true,
                hash_algorithm: "sha256".to_string(),
            },
            templates: TemplateConfig {
                template_dir: None,
                settings: template_settings,
            },
            output: OutputConfig {
                format: "markdown".to_string(),
                include_metadata: true,
                static_site_compatible: true,
            },
            llm: LlmConfig {
                enabled: false,
                provider: "rig-openai".to_string(),
                model: "gpt-4".to_string(),
                api_key: None,
                base_url: None,
                max_tokens: Some(2000),
                temperature: Some(0.3),
            },
        }
    }
}

impl Config {
    /// Load configuration from file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)
            .map_err(|e| CodesworthError::Config(e.to_string()))?;
        Ok(config)
    }

    /// Save configuration to file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| CodesworthError::Config(e.to_string()))?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Load configuration with fallback to default
    pub fn load_or_default<P: AsRef<Path>>(path: Option<P>) -> Result<Self> {
        match path {
            Some(p) => {
                if p.as_ref().exists() {
                    Self::load(p)
                } else {
                    Ok(Self::default())
                }
            }
            None => {
                // Try common config file locations
                let candidates = [
                    "Codesworth.toml",
                    "codesworth.toml",
                    ".codesworth.toml",
                ];

                for candidate in &candidates {
                    if Path::new(candidate).exists() {
                        return Self::load(candidate);
                    }
                }

                Ok(Self::default())
            }
        }
    }
}