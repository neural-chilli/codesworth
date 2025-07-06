use std::path::{Path, PathBuf};
use anyhow::Result;
use tracing::{info, warn, debug};

use crate::config::Config;
use crate::error::CodesworthError;
use super::{
    CodeParser, DocGenerator, ContentDiffer,
    EditProtector, DocValidator, LlmDocumenter
};

/// Main orchestration engine for Codesworth
///
/// The Engine coordinates all the major components:
/// - Configuration management
/// - Code parsing and analysis
/// - Documentation generation
/// - Edit preservation
/// - Content synchronization
/// - LLM-powered enhancement
pub struct Engine {
    config: Config,
    parser: CodeParser,
    generator: DocGenerator,
    differ: ContentDiffer,
    protector: EditProtector,
    validator: DocValidator,
    llm_documenter: Option<Box<dyn LlmDocumenter>>,
}

impl Engine {
    /// Create a new engine instance
    pub async fn new(config_path: Option<&Path>) -> Result<Self> {
        let config = Config::load_or_default(config_path)?;

        debug!("Loaded configuration: {:?}", config);

        let parser = CodeParser::new(&config.parsing)?;
        let generator = DocGenerator::new(&config.generation, &config.templates)?;
        let differ = ContentDiffer::new(&config.generation.hash_algorithm)?;
        let protector = EditProtector::new();
        let validator = DocValidator::new(&config)?;

        // Initialize LLM documenter if enabled
        let llm_documenter = if config.llm.enabled {
            match super::llm::create_documenter(&config.llm) {
                Ok(documenter) => {
                    info!("✅ LLM integration enabled: {}", documenter.provider_name());
                    Some(documenter)
                }
                Err(e) => {
                    warn!("⚠️ Failed to initialize LLM documenter: {}", e);
                    warn!("Continuing without LLM enhancement");
                    None
                }
            }
        } else {
            debug!("LLM integration disabled");
            None
        };

        Ok(Self {
            config,
            parser,
            generator,
            differ,
            protector,
            validator,
            llm_documenter,
        })
    }

    /// Initialize documentation structure
    pub async fn init(&self, path: Option<PathBuf>, non_interactive: bool) -> Result<()> {
        let target_dir = path.unwrap_or_else(|| std::env::current_dir().unwrap());

        info!("Initializing Codesworth in: {}", target_dir.display());

        // Create directory structure
        self.create_directory_structure(&target_dir).await?;

        // Create default configuration
        if !non_interactive {
            self.create_interactive_config(&target_dir).await?;
        } else {
            self.create_default_config(&target_dir).await?;
        }

        info!("✅ Codesworth initialized successfully!");
        Ok(())
    }

    /// Generate initial documentation
    pub async fn generate(&mut self, source: Option<PathBuf>, output: Option<PathBuf>, force: bool) -> Result<()> {
        let source_dir = source.unwrap_or_else(|| self.config.project.source_dirs[0].clone());
        let output_dir = output.unwrap_or_else(|| self.config.project.docs_dir.clone());

        info!("Generating documentation from {} to {}", source_dir.display(), output_dir.display());

        // Parse source code
        let parsed_files = self.parser.parse_directory(&source_dir).await?;
        info!("Parsed {} files", parsed_files.len());

        // Generate documentation for each file
        for parsed_file in parsed_files {
            let output_path = self.determine_output_path(&parsed_file, &output_dir)?;

            // Check if we should regenerate
            if !force && output_path.exists() {
                let should_regenerate = self.should_regenerate(&parsed_file, &output_path).await?;
                if !should_regenerate {
                    debug!("Skipping {}, no changes detected", output_path.display());
                    continue;
                }
            }

            // Generate new documentation
            let generated_doc = if let Some(ref llm) = self.llm_documenter {
                self.generator.generate_with_llm(&parsed_file, Some(llm.as_ref())).await?
            } else {
                self.generator.generate(&parsed_file).await?
            };

            // Preserve existing edits if file exists
            let final_content = if output_path.exists() && self.config.generation.preserve_edits {
                let existing_content = std::fs::read_to_string(&output_path)?;
                self.protector.merge_with_existing(&generated_doc.content, &existing_content)?
            } else {
                generated_doc.content
            };

            // Write the documentation
            if let Some(parent) = output_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&output_path, final_content)?;

            info!("Generated: {}", output_path.display());
        }

        Ok(())
    }

    /// Update only changed sections
    pub async fn sync(&self, dry_run: bool, fail_on_changes: bool) -> Result<()> {
        info!("Synchronizing documentation...");

        let changes = self.detect_changes().await?;

        if changes.is_empty() {
            info!("✅ Documentation is up to date");
            return Ok(());
        }

        info!("Found {} files that need updates", changes.len());

        if dry_run {
            for change in &changes {
                println!("Would update: {}", change.display());
            }
            return Ok(());
        }

        if fail_on_changes {
            return Err(CodesworthError::Validation(
                format!("Documentation is out of sync. {} files need updates", changes.len())
            ).into());
        }

        // Perform the actual sync
        for file_path in changes {
            self.sync_single_file(&file_path).await?;
            info!("Updated: {}", file_path.display());
        }

        Ok(())
    }

    /// Validate documentation health
    pub async fn validate(&self, strict: bool) -> Result<()> {
        info!("Validating documentation...");

        let result = self.validator.validate_all(&self.config.project.docs_dir, strict).await?;

        if result.is_valid() {
            info!("✅ Documentation validation passed");
        } else {
            warn!("❌ Documentation validation failed:");
            for error in &result.errors {
                warn!("  - {}", error);
            }
            if strict {
                return Err(CodesworthError::Validation("Validation failed in strict mode".to_string()).into());
            }
        }

        Ok(())
    }

    /// Export for static sites
    pub async fn publish(&self, format: &str, output: Option<PathBuf>) -> Result<()> {
        info!("Publishing documentation in {} format", format);

        let output_dir = output.unwrap_or_else(|| PathBuf::from(format!("publish-{}", format)));

        match format {
            "hugo" => self.publish_hugo(&output_dir).await,
            "jekyll" => self.publish_jekyll(&output_dir).await,
            "gitbook" => self.publish_gitbook(&output_dir).await,
            _ => Err(CodesworthError::Config(format!("Unsupported format: {}", format)).into()),
        }
    }

    // Private helper methods

    async fn create_directory_structure(&self, target_dir: &Path) -> Result<()> {
        let docs_dir = target_dir.join(&self.config.project.docs_dir);

        info!("Creating directory structure in {}", docs_dir.display());

        std::fs::create_dir_all(&docs_dir)?;
        std::fs::create_dir_all(docs_dir.join("architecture"))?;
        std::fs::create_dir_all(docs_dir.join("services"))?;
        std::fs::create_dir_all(docs_dir.join("guides"))?;
        std::fs::create_dir_all(docs_dir.join("decisions"))?;

        // Create some example files
        let readme_content = r#"# Project Documentation

This documentation was generated and is maintained by Codesworth.

## Structure

- **architecture/**: System overviews and architectural decisions
- **services/**: Per-service documentation with API details
- **guides/**: Human-authored tutorials and how-to guides
- **decisions/**: Architectural Decision Records (ADRs)

## Using Protected Regions

You can preserve your edits across regenerations using protected regions:

```markdown
<!-- PROTECTED: Important Note -->
This content will never be overwritten by Codesworth.
Use this for architectural decisions, important context,
or anything else that needs human curation.
<!-- /PROTECTED -->
```

## Commands

- `codesworth generate` - Generate documentation from source code
- `codesworth sync` - Update only changed sections
- `codesworth validate` - Check documentation health
- `codesworth publish` - Export for static sites
"#;

        std::fs::write(docs_dir.join("README.md"), readme_content)?;

        let arch_example = r#"# System Architecture

<!-- PROTECTED: Architecture Overview -->
Add your high-level system architecture description here.
Consider including:
- System boundaries and responsibilities
- Key architectural patterns
- Technology choices and rationale
- Data flow and service interactions
<!-- /PROTECTED -->

## Services

*Service documentation will be generated automatically from source code.*

## Data Flow

<!-- PROTECTED: Data Flow -->
Describe how data flows through your system:
- Input sources and formats
- Processing pipelines
- Storage mechanisms
- Output destinations
<!-- /PROTECTED -->
"#;

        std::fs::write(docs_dir.join("architecture").join("README.md"), arch_example)?;

        let adr_example = r#"# Architectural Decision Records

This directory contains records of architectural decisions made for this project.

## Format

Each ADR should follow this format:

```markdown
# ADR-001: [Decision Title]

**Status**: [Proposed | Accepted | Deprecated | Superseded]
**Date**: YYYY-MM-DD
**Deciders**: [List of people involved]

## Context

What is the issue that we're seeing that is motivating this decision or change?

## Decision

What is the change that we're proposing or have agreed to implement?

## Consequences

What becomes easier or more difficult to do and any risks introduced by the change?
```

## Index

*ADRs will be listed here as they are created.*
"#;

        std::fs::write(docs_dir.join("decisions").join("README.md"), adr_example)?;

        info!("✅ Created documentation structure");
        Ok(())
    }

    async fn create_default_config(&self, target_dir: &Path) -> Result<()> {
        let config_path = target_dir.join("Codesworth.toml");

        // Create a config tailored to the current directory
        let mut config = self.config.clone();

        // Try to detect project name from directory
        if let Some(dir_name) = target_dir.file_name().and_then(|n| n.to_str()) {
            config.project.name = dir_name.to_string();
        }

        // Look for common source directories
        let potential_source_dirs = vec!["src", "lib", "app", "source"];
        let mut found_source_dirs = Vec::new();

        for dir in potential_source_dirs {
            let path = target_dir.join(dir);
            if path.exists() && path.is_dir() {
                found_source_dirs.push(PathBuf::from(dir));
            }
        }

        if !found_source_dirs.is_empty() {
            config.project.source_dirs = found_source_dirs;
        }

        config.save(config_path)?;
        info!("✅ Created Codesworth.toml configuration");
        Ok(())
    }

    async fn create_interactive_config(&self, target_dir: &Path) -> Result<()> {
        // TODO: Implement interactive configuration
        // For now, just create default config
        self.create_default_config(target_dir).await
    }

    fn determine_output_path(&self, parsed_file: &super::ParsedFile, output_dir: &Path) -> Result<PathBuf> {
        // Convert source file path to documentation path
        let relative_path = parsed_file.path.strip_prefix(&self.config.project.source_dirs[0])
            .map_err(|e| CodesworthError::FileSystem(e.to_string()))?;

        let mut doc_path = output_dir.join(relative_path);
        doc_path.set_extension("md");

        Ok(doc_path)
    }

    async fn should_regenerate(&self, parsed_file: &super::ParsedFile, output_path: &Path) -> Result<bool> {
        if !output_path.exists() {
            return Ok(true);
        }

        let existing_content = std::fs::read_to_string(output_path)?;
        let content_changed = self.differ.has_content_changed(&parsed_file.content_hash, &existing_content)?;

        Ok(content_changed)
    }

    async fn detect_changes(&self) -> Result<Vec<PathBuf>> {
        // TODO: Implement change detection logic
        // This would compare file modification times, content hashes, etc.
        Ok(vec![])
    }

    async fn sync_single_file(&self, _file_path: &Path) -> Result<()> {
        // TODO: Implement single file sync
        Ok(())
    }

    async fn publish_hugo(&self, _output_dir: &Path) -> Result<()> {
        // TODO: Implement Hugo export
        info!("Hugo export not yet implemented");
        Ok(())
    }

    async fn publish_jekyll(&self, _output_dir: &Path) -> Result<()> {
        // TODO: Implement Jekyll export
        info!("Jekyll export not yet implemented");
        Ok(())
    }

    async fn publish_gitbook(&self, _output_dir: &Path) -> Result<()> {
        // TODO: Implement GitBook export
        info!("GitBook export not yet implemented");
        Ok(())
    }
}