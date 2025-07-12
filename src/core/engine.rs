use std::path::{Path, PathBuf};
use anyhow::Result;
use tracing::{info, warn, debug};

use crate::config::Config;
use crate::error::CodesworthError;
use super::{
    CodeParser, DocGenerator, ContentDiffer, EditProtector, DocValidator, LlmDocumenter,
    PackageAnalyzer, BatchProcessor, ContextScanner, PackageAnalysis,
    BatchDocumentationRequest, HumanContext, SystemContext, AnalysisFocus,
    FocusArea, DepthLevel, TargetAudience, hierarchical_analyzer::HierarchicalAnalyzer,
    system_overview_generator::SystemOverviewGenerator
};

/// Main orchestration engine for Codesworth with package-level processing
pub struct Engine {
    config: Config,
    parser: CodeParser,
    generator: DocGenerator,
    differ: ContentDiffer,
    protector: EditProtector,
    validator: DocValidator,
    llm_documenter: Option<Box<dyn LlmDocumenter>>,
    package_analyzer: PackageAnalyzer,
    batch_processor: BatchProcessor,
    context_scanner: ContextScanner,
    system_overview_generator: SystemOverviewGenerator,
}

impl Engine {
    /// Create a new engine instance with package-level capabilities
    pub async fn new(config_path: Option<&Path>) -> Result<Self> {
        let config = Config::load_or_default(config_path)?;

        debug!("Loaded configuration: {:?}", config);

        let parser = CodeParser::new(&config.parsing)?;
        let generator = DocGenerator::new(&config.generation, &config.templates)?;
        let differ = ContentDiffer::new(&config.generation.hash_algorithm)?;
        let protector = EditProtector::new();
        let validator = DocValidator::new(&config)?;
        let package_analyzer = PackageAnalyzer::new(&config.parsing);
        let batch_processor = BatchProcessor::new();
        let context_scanner = ContextScanner::new().map_err(|e| anyhow::anyhow!("Failed to create context scanner: {}", e))?;

        // Determine context window limit based on LLM config
        let context_window_limit = config.llm.max_tokens.unwrap_or(8000) as usize;
        let system_overview_generator = SystemOverviewGenerator::new(context_window_limit);

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
            package_analyzer,
            batch_processor,
            context_scanner,
            system_overview_generator,
        })
    }

    /// Generate initial documentation using the new package-level approach
    pub async fn generate(&mut self, source: Option<PathBuf>, output: Option<PathBuf>, force: bool) -> Result<()> {
        let source_dir = source.unwrap_or_else(|| self.config.project.source_dirs[0].clone());
        let output_dir = output.unwrap_or_else(|| self.config.project.docs_dir.clone());

        info!("🔍 Analyzing project structure for package-level documentation...");
        info!("Source: {}", source_dir.display());
        info!("Output: {}", output_dir.display());

        // Step 1: Scan for human context
        let project_root = source_dir.parent().unwrap_or(&source_dir);
        info!("📖 Scanning for human-authored context in {}", project_root.display());
        let human_context = self.context_scanner.scan_project_context(&project_root).await
            .map_err(|e| anyhow::anyhow!("Failed to scan project context: {}", e))?;

        info!("Found context: README={}, Architecture docs={}, ADRs={}, Comments={}",
            human_context.readme_content.is_some(),
            human_context.architecture_docs.len(),
            human_context.adrs.len(),
            human_context.inline_comments.len()
        );

        // Step 2: Analyze packages
        info!("🏗️ Analyzing packages and dependencies...");
        let package_analyses = self.package_analyzer
            .analyze_directory(&source_dir, &mut self.parser)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to analyze packages: {}", e))?;

        info!("Found {} packages:", package_analyses.len());
        for pkg in &package_analyses {
            info!("  - {} (complexity: {:.2}, files: {})",
                pkg.package_name, pkg.complexity_score(), pkg.files.len());
        }

        // Step 3: Build system context
        let package_names: Vec<_> = package_analyses.iter().map(|p| p.package_name.clone()).collect();
        let system_context = self.context_scanner
            .scan_system_context(&project_root, &package_names)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to scan system context: {}", e))?;

        // Step 4: Generate documentation for each package
        for package_analysis in &package_analyses {
            let should_regenerate = force || self.should_regenerate_package(&package_analysis, &output_dir).await?;

            if !should_regenerate {
                debug!("Skipping {}, no changes detected", package_analysis.package_name);
                continue;
            }

            info!("📝 Generating documentation for package: {}", package_analysis.package_name);

            if package_analysis.needs_priority_documentation() {
                info!("⚡ High priority package - using enhanced documentation");
            }

            let output_path = self.determine_package_output_path(&package_analysis, &output_dir)?;

            // Generate using batch processing if LLM is available
            let final_content = if let Some(ref llm) = self.llm_documenter {
                self.generate_enhanced_package_docs(&package_analysis, &human_context, &system_context, llm.as_ref()).await?
            } else {
                self.generate_basic_package_docs(&package_analysis).await?
            };

            // Preserve existing edits if file exists
            let final_content = if output_path.exists() && self.config.generation.preserve_edits {
                let existing_content = std::fs::read_to_string(&output_path)?;
                self.protector.merge_with_existing(&final_content, &existing_content)
                    .map_err(|e| anyhow::anyhow!("Failed to merge existing edits: {}", e))?
            } else {
                final_content
            };

            // Write the documentation
            if let Some(parent) = output_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&output_path, final_content)?;

            info!("✅ Generated: {}", output_path.display());
        }

        // Step 5: Generate comprehensive system-level overview
        info!("📋 Generating comprehensive system overview...");
        self.generate_comprehensive_system_overview(&package_analyses, &human_context, &system_context, &output_dir).await?;

        info!("🎉 Documentation generation complete!");
        Ok(())
    }

    async fn generate_enhanced_package_docs(
        &self,
        package_analysis: &PackageAnalysis,
        human_context: &HumanContext,
        system_context: &SystemContext,
        llm_documenter: &dyn LlmDocumenter,
    ) -> Result<String> {
        // Build comprehensive request for batch processing
        let request = BatchDocumentationRequest {
            package_analysis: package_analysis.clone(),
            human_context: human_context.clone(),
            system_context: system_context.clone(),
            enhancement_focus: AnalysisFocus {
                focus_areas: vec![
                    FocusArea::Purpose,
                    FocusArea::Architecture,
                    FocusArea::Integrations,
                    FocusArea::Maintenance,
                ],
                depth_level: if package_analysis.needs_priority_documentation() {
                    DepthLevel::Detailed
                } else {
                    DepthLevel::Overview
                },
                target_audience: TargetAudience::NewTeamMember,
            },
        };

        info!("🧠 Enhancing with LLM: {} tokens of context",
            self.estimate_context_size(&request));

        let response = self.batch_processor
            .process_package(request, llm_documenter)
            .await
            .map_err(|e| anyhow::anyhow!("LLM processing failed: {}", e))?;

        info!("✨ LLM enhancement complete (confidence: {:.2})",
            response.metadata.confidence_score);

        // Format the enhanced response into markdown
        self.format_enhanced_response(&response, package_analysis).await
    }

    async fn generate_basic_package_docs(&self, package_analysis: &PackageAnalysis) -> Result<String> {
        // Generate basic documentation without LLM enhancement
        let mut content = String::new();

        // Header with metadata
        content.push_str(&format!(
            "---\n\
            package: {}\n\
            generated_from: {}\n\
            last_updated: {}\n\
            package_hash: {}\n\
            complexity_score: {:.2}\n\
            ---\n\n",
            package_analysis.package_name,
            package_analysis.package_path.display(),
            chrono::Utc::now().to_rfc3339(),
            package_analysis.package_hash,
            package_analysis.complexity_score()
        ));

        // Package overview
        content.push_str(&format!("# {}\n\n", package_analysis.package_name));

        if let Some(docs) = &package_analysis.package_docs {
            content.push_str(docs);
            content.push_str("\n\n");
        } else {
            content.push_str(&format!(
                "<!-- PROTECTED: Package Overview -->\n\
                {}\n\n\
                Add a description of what this package does and why it exists.\n\
                Consider explaining the architectural decisions and design patterns used.\n\
                <!-- /PROTECTED -->\n\n",
                package_analysis.generate_summary()
            ));
        }

        // Public API
        if !package_analysis.public_api.functions.is_empty() || !package_analysis.public_api.types.is_empty() {
            content.push_str("## Public API\n\n");

            // Entry points
            if !package_analysis.public_api.entry_points.is_empty() {
                content.push_str("### Entry Points\n\n");
                for entry_point in &package_analysis.public_api.entry_points {
                    content.push_str(&format!(
                        "- **{}** ({:?}): {}\n",
                        entry_point.name,
                        entry_point.entry_type,
                        entry_point.description.as_deref().unwrap_or("No description")
                    ));
                }
                content.push_str("\n");
            }

            // Types
            if !package_analysis.public_api.types.is_empty() {
                content.push_str("### Types\n\n");
                for api_type in &package_analysis.public_api.types {
                    content.push_str(&format!(
                        "#### {} ({})\n\n{}\n\n",
                        api_type.name,
                        api_type.type_kind,
                        api_type.docs.as_deref().unwrap_or("*No documentation available*")
                    ));
                }
            }

            // Functions
            if !package_analysis.public_api.functions.is_empty() {
                content.push_str("### Functions\n\n");
                for function in &package_analysis.public_api.functions {
                    content.push_str(&format!(
                        "#### {}\n\n```\n{}\n```\n\n{}\n\n",
                        function.name,
                        function.signature,
                        function.docs.as_deref().unwrap_or("*No documentation available*")
                    ));
                }
            }
        }

        // Dependencies
        if !package_analysis.dependencies.external_deps.is_empty() {
            content.push_str("## Dependencies\n\n");
            for dep in &package_analysis.dependencies.external_deps {
                content.push_str(&format!("- **{}**: {}\n", dep.name, dep.usage_context));
            }
            content.push_str("\n");
        }

        // Important considerations
        if !package_analysis.complexity_indicators.gotchas.is_empty() {
            content.push_str("## Important Considerations\n\n");
            for gotcha in &package_analysis.complexity_indicators.gotchas {
                content.push_str(&format!(
                    "### {:?}: {}\n\n{}\n\n",
                    gotcha.category,
                    gotcha.description,
                    gotcha.suggestion.as_deref().unwrap_or("No specific recommendation")
                ));
            }
        }

        // Implementation notes section
        content.push_str(
            "## Implementation Details\n\n\
            <!-- PROTECTED: Implementation Notes -->\n\
            Add notes about implementation decisions, performance considerations,\n\
            error handling strategies, or anything else that would be useful\n\
            for maintainers.\n\
            <!-- /PROTECTED -->\n\n"
        );

        // Testing section
        content.push_str(
            "## Testing\n\n\
            <!-- PROTECTED: Testing Strategy -->\n\
            Describe the testing approach for this package, including:\n\
            - Unit test coverage\n\
            - Integration test scenarios\n\
            - Mock strategies\n\
            - Performance test requirements\n\
            <!-- /PROTECTED -->\n\n"
        );

        content.push_str("---\n\n*This documentation was generated by Codesworth. Protected sections are preserved across regenerations.*\n");

        Ok(content)
    }

    async fn format_enhanced_response(
        &self,
        response: &super::BatchDocumentationResponse,
        package_analysis: &PackageAnalysis,
    ) -> Result<String> {
        let mut content = String::new();

        // Header with metadata
        content.push_str(&format!(
            "---\n\
            package: {}\n\
            generated_from: {}\n\
            last_updated: {}\n\
            package_hash: {}\n\
            complexity_score: {:.2}\n\
            llm_enhanced: true\n\
            confidence_score: {:.2}\n\
            ---\n\n",
            package_analysis.package_name,
            package_analysis.package_path.display(),
            chrono::Utc::now().to_rfc3339(),
            package_analysis.package_hash,
            package_analysis.complexity_score(),
            response.metadata.confidence_score
        ));

        // Main content from LLM
        content.push_str(&response.package_overview);

        // Add cross-references if any
        if !response.cross_references.is_empty() {
            content.push_str("\n\n## Related Packages\n\n");
            for cross_ref in &response.cross_references {
                content.push_str(&format!(
                    "- **{}** ({}): {}\n",
                    cross_ref.target_package,
                    cross_ref.relationship,
                    cross_ref.description
                ));
            }
        }

        content.push_str("\n\n---\n\n*This documentation was generated by Codesworth with LLM enhancement. Protected sections are preserved across regenerations.*\n");

        Ok(content)
    }

    async fn generate_comprehensive_system_overview(
        &self,
        package_analyses: &[PackageAnalysis],
        human_context: &HumanContext,
        system_context: &SystemContext,
        output_dir: &Path,
    ) -> Result<()> {
        info!("🎯 Generating comprehensive system overview with full analysis...");

        if let Some(ref llm) = self.llm_documenter {
            // Use the system overview generator for comprehensive analysis
            let overview = self.system_overview_generator
                .generate_system_overview(package_analyses, human_context, system_context, llm.as_ref())
                .await?;

            // Write the comprehensive overview
            let overview_path = output_dir.join("README.md");
            self.system_overview_generator.write_system_overview(&overview, &overview_path).await?;

            info!("✅ Comprehensive system overview generated: {}", overview_path.display());
        } else {
            // Fallback to basic system overview without LLM
            self.generate_system_overview(package_analyses, human_context, output_dir).await?;
        }

        Ok(())
    }

    /// Generate basic system overview (fallback without LLM)
    async fn generate_system_overview(
        &self,
        package_analyses: &[PackageAnalysis],
        human_context: &HumanContext,
        output_dir: &Path,
    ) -> Result<()> {
        let overview_path = output_dir.join("README.md");

        let mut content = String::new();
        content.push_str("# System Documentation\n\n");

        if let Some(readme) = &human_context.readme_content {
            content.push_str(readme);
            content.push_str("\n\n");
        }

        content.push_str("## Package Overview\n\n");
        content.push_str("This system is organized into the following packages:\n\n");

        for pkg in package_analyses {
            content.push_str(&format!(
                "### [{}]({})\n\n{}\n\n- **Complexity**: {:.2}\n- **Files**: {}\n- **Public API**: {} functions, {} types\n\n",
                pkg.package_name,
                format!("{}/README.md", pkg.package_name),
                pkg.generate_summary(),
                pkg.complexity_score(),
                pkg.files.len(),
                pkg.public_api.functions.len(),
                pkg.public_api.types.len()
            ));
        }

        // Preserve existing system overview edits
        let final_content = if overview_path.exists() && self.config.generation.preserve_edits {
            let existing_content = std::fs::read_to_string(&overview_path)?;
            self.protector.merge_with_existing(&content, &existing_content)
                .map_err(|e| anyhow::anyhow!("Failed to merge system overview: {}", e))?
        } else {
            content
        };

        std::fs::write(&overview_path, final_content)?;
        Ok(())
    }

    // Helper methods

    fn determine_package_output_path(&self, package_analysis: &PackageAnalysis, output_dir: &Path) -> Result<PathBuf> {
        let package_output_dir = output_dir.join(&package_analysis.package_name);
        Ok(package_output_dir.join("README.md"))
    }

    async fn should_regenerate_package(&self, package_analysis: &PackageAnalysis, output_dir: &Path) -> Result<bool> {
        let output_path = self.determine_package_output_path(package_analysis, output_dir)?;

        if !output_path.exists() {
            return Ok(true);
        }

        // Check if package hash has changed
        let existing_content = std::fs::read_to_string(&output_path)?;

        // Extract hash from frontmatter if present
        if let Some(hash_line) = existing_content.lines().find(|line| line.starts_with("package_hash:")) {
            let existing_hash = hash_line.trim_start_matches("package_hash:").trim();
            return Ok(existing_hash != package_analysis.package_hash);
        }

        // Fallback to checking modification time
        let metadata = std::fs::metadata(&output_path)?;
        let doc_modified = metadata.modified()?;

        for file in &package_analysis.files {
            if file.modified_time > doc_modified {
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn estimate_context_size(&self, request: &BatchDocumentationRequest) -> usize {
        // Rough estimate of context size in tokens (4 chars ≈ 1 token)
        let mut size = 0;

        if let Some(readme) = &request.human_context.readme_content {
            size += readme.len() / 4;
        }

        size += request.human_context.architecture_docs.iter()
            .map(|doc| doc.content.len() / 4)
            .sum::<usize>();

        size += request.package_analysis.files.iter()
            .map(|file| file.source_content.len() / 20) // Much smaller sample of source
            .sum::<usize>();

        size
    }

    // Implement the CLI interface methods (delegating to the new system)

    pub async fn init(&self, path: Option<PathBuf>, non_interactive: bool) -> Result<()> {
        // Keep existing init logic for now
        let target_dir = path.unwrap_or_else(|| std::env::current_dir().unwrap());
        info!("Initializing Codesworth in: {}", target_dir.display());

        // Use the original implementation for now
        Ok(())
    }

    pub async fn sync(&self, dry_run: bool, fail_on_changes: bool) -> Result<()> {
        info!("🔄 Synchronizing documentation with package-level analysis...");

        if dry_run {
            info!("📋 Dry run mode - showing what would be updated");
        }

        // TODO: Implement sync with new package approach
        Ok(())
    }

    pub async fn validate(&self, strict: bool) -> Result<()> {
        info!("✅ Validating documentation...");

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

    pub async fn publish(&self, format: &str, output: Option<PathBuf>) -> Result<()> {
        info!("📤 Publishing documentation in {} format", format);

        let output_dir = output.unwrap_or_else(|| PathBuf::from(format!("publish-{}", format)));

        match format {
            "hugo" => self.publish_hugo(&output_dir).await,
            "jekyll" => self.publish_jekyll(&output_dir).await,
            "gitbook" => self.publish_gitbook(&output_dir).await,
            _ => Err(CodesworthError::Config(format!("Unsupported format: {}", format)).into()),
        }
    }

    async fn publish_hugo(&self, _output_dir: &Path) -> Result<()> {
        info!("Hugo export not yet implemented");
        Ok(())
    }

    async fn publish_jekyll(&self, _output_dir: &Path) -> Result<()> {
        info!("Jekyll export not yet implemented");
        Ok(())
    }

    async fn publish_gitbook(&self, _output_dir: &Path) -> Result<()> {
        info!("GitBook export not yet implemented");
        Ok(())
    }
}