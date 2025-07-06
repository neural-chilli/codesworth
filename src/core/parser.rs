use std::path::{Path, PathBuf};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use walkdir::WalkDir;
use ignore::WalkBuilder;

use crate::config::ParsingConfig;
use crate::error::{CodesworthError, Result};
use super::languages::{LanguageParser, RustParser, JavaParser, PythonParser, CSharpParser, JavaScriptParser};

/// Represents a parsed source file with extracted metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedFile {
    /// File path relative to project root
    pub path: PathBuf,

    /// Programming language detected
    pub language: String,

    /// Content hash for change detection
    pub content_hash: String,

    /// Last modification time
    pub modified_time: std::time::SystemTime,

    /// Extracted modules/functions/types
    pub modules: Vec<ParsedModule>,

    /// File-level documentation/comments
    pub file_docs: Option<String>,

    /// Raw source content (for template context)
    pub source_content: String,
}

/// Represents a parsed module, function, struct, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedModule {
    /// Name of the item
    pub name: String,

    /// Type of item (module, function, struct, enum, etc.)
    pub item_type: String,

    /// Visibility (pub, pub(crate), private)
    pub visibility: String,

    /// Documentation comments
    pub docs: Option<String>,

    /// Function signature or type definition
    pub signature: Option<String>,

    /// Line range in source file
    pub line_range: (usize, usize),

    /// Child items (for modules containing functions, etc.)
    pub children: Vec<ParsedModule>,
}

/// Multi-language code parser that delegates to language-specific parsers
pub struct CodeParser {
    config: ParsingConfig,
    language_parsers: HashMap<String, Box<dyn LanguageParser>>,
}

impl CodeParser {
    pub fn new(config: &ParsingConfig) -> Result<Self> {
        let mut language_parsers: HashMap<String, Box<dyn LanguageParser>> = HashMap::new();

        // Initialize language parsers based on configuration
        for language in &config.languages {
            match language.as_str() {
                "rust" => {
                    let rust_parser = RustParser::new()?;
                    language_parsers.insert("rust".to_string(), Box::new(rust_parser));
                }
                "java" => {
                    let java_parser = JavaParser::new()?;
                    language_parsers.insert("java".to_string(), Box::new(java_parser));
                }
                "python" => {
                    let python_parser = PythonParser::new()?;
                    language_parsers.insert("python".to_string(), Box::new(python_parser));
                }
                "csharp" => {
                    let csharp_parser = CSharpParser::new()?;
                    language_parsers.insert("csharp".to_string(), Box::new(csharp_parser));
                }
                "javascript" => {
                    let javascript_parser = JavaScriptParser::new()?;
                    language_parsers.insert("javascript".to_string(), Box::new(javascript_parser));
                }
                _ => {
                    // For now, skip unsupported languages
                    continue;
                }
            }
        }

        Ok(Self {
            config: config.clone(),
            language_parsers,
        })
    }

    /// Parse all files in a directory
    pub async fn parse_directory<P: AsRef<Path>>(&mut self, dir: P) -> Result<Vec<ParsedFile>> {
        let mut parsed_files = Vec::new();

        // Use ignore crate to respect .gitignore and custom patterns
        let walker = WalkBuilder::new(dir)
            .hidden(false)
            .git_ignore(true)
            .build();

        for entry in walker {
            let entry = entry.map_err(|e| CodesworthError::FileSystem(e.to_string()))?;
            let path = entry.path();

            if path.is_file() && self.should_parse_file(path)? {
                if let Ok(parsed) = self.parse_file(path).await {
                    parsed_files.push(parsed);
                }
            }
        }

        Ok(parsed_files)
    }

    /// Parse a single source file
    pub async fn parse_file<P: AsRef<Path>>(&mut self, file_path: P) -> Result<ParsedFile> {
        let path = file_path.as_ref();
        let language = self.detect_language(path)?;

        // Read file content
        let source_content = std::fs::read_to_string(path)?;

        // Check file size
        if source_content.len() > self.config.max_file_size {
            return Err(CodesworthError::Parser(
                format!("File {} exceeds maximum size limit", path.display())
            ));
        }

        // Calculate content hash
        let content_hash = self.calculate_hash(&source_content);

        // Get modification time
        let metadata = std::fs::metadata(path)?;
        let modified_time = metadata.modified()?;

        // Parse using the appropriate language parser
        let modules = if let Some(parser) = self.language_parsers.get_mut(&language) {
            parser.parse(&source_content, path)?
        } else {
            // Fallback for unsupported languages
            vec![ParsedModule {
                name: path.file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                item_type: "file".to_string(),
                visibility: "public".to_string(),
                docs: None,
                signature: None,
                line_range: (1, source_content.lines().count()),
                children: vec![],
            }]
        };

        // Extract file-level documentation
        let file_docs = if let Some(parser) = self.language_parsers.get(&language) {
            parser.extract_file_docs(&source_content)
        } else {
            None
        };

        Ok(ParsedFile {
            path: path.to_path_buf(),
            language,
            content_hash,
            modified_time,
            modules,
            file_docs,
            source_content,
        })
    }

    /// Determine if a file should be parsed based on configuration
    fn should_parse_file(&self, path: &Path) -> Result<bool> {
        // Check file extension against all registered language parsers
        if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
            for (lang, parser) in &self.language_parsers {
                if self.config.languages.contains(lang) && parser.file_extensions().contains(&extension) {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Detect programming language from file path
    fn detect_language(&self, path: &Path) -> Result<String> {
        if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
            for (lang, parser) in &self.language_parsers {
                if parser.file_extensions().contains(&extension) {
                    return Ok(lang.clone());
                }
            }
        }

        Err(CodesworthError::Parser(
            format!("Could not detect language for file: {}", path.display())
        ))
    }

    /// Calculate SHA256 hash of content
    fn calculate_hash(&self, content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}