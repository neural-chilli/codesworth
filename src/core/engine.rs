// src/core/engine.rs
use std::path::{Path, PathBuf};
use anyhow::Result;
use tracing::{info, warn, debug};

use crate::config::Config;
use crate::error::CodesworthError;
use super::{
    CodeParser, DocGenerator, ContentDiffer, EditProtector, DocValidator, LlmDocumenter,
    PackageAnalyzer, BatchProcessor, ContextScanner, PackageAnalysis,
    BatchDocumentationRequest, HumanContext, SystemContext, AnalysisFocus,
    FocusArea, DepthLevel, TargetAudience, HierarchicalAnalyzer,
    SystemOverviewGenerator, CallChainEngine, CallChainAnalysisResult
};

// Import the BatchDocumentationResponse specifically to avoid confusion
use super::batch_processor::BatchDocumentationResponse;

/// Main orchestration engine for Codesworth with call-chain analysis
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
    call_chain_engine: CallChainEngine,
}

impl Engine {
    /// Create a new engine instance with call-chain capabilities
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

        // Initialize call-chain engine with appropriate parameters
        let call_chain_engine = CallChainEngine::new(6, context_window_limit); // 6 levels deep, full context window

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
            call_chain_engine,
        })
    }

    /// Generate initial documentation using call-chain analysis
    pub async fn generate(&mut self, source: Option<PathBuf>, output: Option<PathBuf>, force: bool) -> Result<()> {
        let source_dir = source.unwrap_or_else(|| self.config.project.source_dirs[0].clone());
        let output_dir = output.unwrap_or_else(|| self.config.project.docs_dir.clone());

        info!("🔍 Starting call-chain analysis for comprehensive documentation...");
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

        // Step 2: Perform comprehensive call-chain analysis
        info!("🔗 Performing call-chain analysis...");
        let call_chain_result = self.call_chain_engine
            .analyze_codebase(&source_dir, &mut self.parser, self.llm_documenter.as_deref())
            .await
            .map_err(|e| anyhow::anyhow!("Call-chain analysis failed: {}", e))?;

        // Log analysis statistics
        let stats = &call_chain_result.stats;
        info!("📊 Call-chain analysis complete:");
        info!("  - {} methods analyzed", stats.total_methods);
        info!("  - {} call chains traced", stats.call_chains_traced);
        info!("  - {} groups created", stats.groups_created);
        info!("  - {} entry points found", stats.entry_points_found);
        if stats.llm_calls_made > 0 {
            info!("  - {} LLM enhancement calls made", stats.llm_calls_made);
        }

        // Step 3: Generate documentation from call-chain analysis
        info!("📝 Generating documentation from call-chain analysis...");
        self.call_chain_engine
            .generate_documentation(&call_chain_result, &output_dir)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to generate call-chain documentation: {}", e))?;

        // Step 4: Generate enhanced package documentation if LLM is available
        if self.llm_documenter.is_some() {
            info!("🧠 Generating enhanced package documentation...");
            self.generate_enhanced_package_docs(&call_chain_result, &human_context, &output_dir).await?;
        }

        // Step 5: Generate comprehensive system overview
        if self.llm_documenter.is_some() {
            info!("📋 Generating comprehensive system overview...");
            self.generate_call_chain_system_overview(&call_chain_result, &human_context, &output_dir).await?;
        }

        info!("🎉 Call-chain documentation generation complete!");
        Ok(())
    }

    /// Generate enhanced package documentation based on call-chain analysis
    async fn generate_enhanced_package_docs(
        &self,
        call_chain_result: &CallChainAnalysisResult,
        human_context: &HumanContext,
        output_dir: &Path,
    ) -> Result<()> {
        if let Some(ref llm) = self.llm_documenter {
            info!("🔍 Analyzing packages for enhanced documentation...");

            // Get unique packages from call chains
            let mut packages_to_analyze = std::collections::HashSet::new();
            for group in &call_chain_result.call_chain_groups {
                for file_path in &group.involved_files {
                    if let Some(package_name) = self.extract_package_name_from_path(file_path) {
                        packages_to_analyze.insert(package_name);
                    }
                }
            }

            info!("Found {} packages to enhance", packages_to_analyze.len());

            // For each unique package, generate enhanced documentation
            for package_name in packages_to_analyze {
                info!("Enhancing package: {}", package_name);

                // Find all groups that involve this package
                let relevant_groups: Vec<_> = call_chain_result.call_chain_groups.iter()
                    .filter(|group| group.involved_files.iter()
                        .any(|file| self.extract_package_name_from_path(file) == Some(package_name.clone())))
                    .collect();

                if !relevant_groups.is_empty() {
                    self.generate_package_docs_from_groups(&package_name, &relevant_groups,
                                                           &call_chain_result.group_analyses, human_context, output_dir, llm.as_ref()).await?;
                }
            }
        }

        Ok(())
    }

    /// Generate package documentation from call-chain groups
    async fn generate_package_docs_from_groups(
        &self,
        package_name: &str,
        groups: &[&super::CallChainGroup],
        analyses: &[super::GroupAnalysis],
        human_context: &HumanContext,
        output_dir: &Path,
        llm_documenter: &dyn LlmDocumenter,
    ) -> Result<()> {
        let mut content = String::new();

        // Header with metadata
        content.push_str(&format!(
            "---\n\
            package: {}\n\
            analysis_type: call_chain_based\n\
            last_updated: {}\n\
            groups_analyzed: {}\n\
            llm_enhanced: true\n\
            ---\n\n",
            package_name,
            chrono::Utc::now().to_rfc3339(),
            groups.len()
        ));

        // Package overview
        content.push_str(&format!("# {}\n\n", package_name));

        // Generate LLM-enhanced overview
        let overview_prompt = self.build_package_overview_prompt(package_name, groups, analyses, human_context);
        let overview_request = super::EnhancementRequest {
            enhancement_type: super::EnhancementType::Custom(overview_prompt),
            context: self.build_minimal_context_for_package(package_name)?,
            current_content: None,
            focus_areas: vec!["purpose".to_string(), "workflows".to_string(), "architecture".to_string()],
        };

        if let Ok(overview_response) = llm_documenter.enhance_documentation(overview_request).await {
            content.push_str(&overview_response.content);
            content.push_str("\n\n");
        }

        // Execution paths section
        content.push_str("## Key Execution Paths\n\n");
        content.push_str("This package participates in the following execution workflows:\n\n");

        for (i, group) in groups.iter().enumerate() {
            if let Some(analysis) = analyses.get(i) {
                content.push_str(&format!("### {}\n\n",
                                          super::CallChainGrouper::default().get_group_name(group)));  // Use default instance
                content.push_str(&format!("**Purpose**: {}\n\n", analysis.description));

                // Show entry points for this group
                let entry_points: std::collections::HashSet<_> = group.call_chains.iter()
                    .map(|chain| chain.entry_point.signature.display_name())
                    .collect();

                if !entry_points.is_empty() {
                    content.push_str("**Entry Points**:\n");
                    for entry_point in &entry_points {
                        content.push_str(&format!("- {}\n", entry_point));
                    }
                    content.push_str("\n");
                }

                content.push_str(&format!("[View detailed call chain analysis](../groups/{}.md)\n\n", group.group_id));
            }
        }

        // Integration points
        content.push_str("## Integration Points\n\n");
        content.push_str("<!-- PROTECTED: Integration Notes -->\n");
        content.push_str("Add notes about how this package integrates with other system components,\n");
        content.push_str("external services, or APIs.\n");
        content.push_str("<!-- /PROTECTED -->\n\n");

        // Implementation considerations
        content.push_str("## Implementation Considerations\n\n");
        content.push_str("<!-- PROTECTED: Implementation Notes -->\n");
        content.push_str("Add notes about:\n");
        content.push_str("- Performance characteristics\n");
        content.push_str("- Error handling strategies\n");
        content.push_str("- Configuration requirements\n");
        content.push_str("- Testing approaches\n");
        content.push_str("<!-- /PROTECTED -->\n\n");

        // Write the enhanced package documentation
        let package_output_dir = output_dir.join("packages").join(package_name);
        std::fs::create_dir_all(&package_output_dir)?;

        let package_file = package_output_dir.join("README.md");

        // Preserve existing edits if file exists
        let final_content = if package_file.exists() && self.config.generation.preserve_edits {
            let existing_content = std::fs::read_to_string(&package_file)?;
            self.protector.merge_with_existing(&content, &existing_content)
                .map_err(|e| anyhow::anyhow!("Failed to merge package edits: {}", e))?
        } else {
            content
        };

        std::fs::write(&package_file, final_content)?;
        info!("✅ Enhanced package documentation: {}", package_file.display());

        Ok(())
    }

    /// Generate system overview from call-chain analysis
    async fn generate_call_chain_system_overview(
        &self,
        call_chain_result: &CallChainAnalysisResult,
        human_context: &HumanContext,
        output_dir: &Path,
    ) -> Result<()> {
        if let Some(ref llm) = self.llm_documenter {
            // Build prompt for system overview based on call-chain analysis
            let system_prompt = self.build_system_overview_prompt(call_chain_result, human_context);

            let overview_request = super::EnhancementRequest {
                enhancement_type: super::EnhancementType::Custom(system_prompt),
                context: self.build_minimal_context_for_system()?,
                current_content: human_context.readme_content.clone(),
                focus_areas: vec!["system_purpose".to_string(), "workflows".to_string(), "architecture".to_string()],
            };

            if let Ok(overview_response) = llm.enhance_documentation(overview_request).await {
                let overview_path = output_dir.join("README.md");

                // Build comprehensive overview content
                let mut content = String::new();
                content.push_str("# System Overview - Call Chain Analysis\n\n");
                content.push_str(&overview_response.content);
                content.push_str("\n\n");

                // Add call-chain specific sections
                content.push_str("## System Architecture\n\n");
                content.push_str(&format!("This system has been analyzed using call-chain analysis, revealing:\n\n"));
                content.push_str(&format!("- **{}** entry points across the system\n", call_chain_result.entry_points.len()));
                content.push_str(&format!("- **{}** execution path groups\n", call_chain_result.call_chain_groups.len()));
                content.push_str(&format!("- **{}** total execution chains\n", call_chain_result.call_chains.len()));
                content.push_str(&format!("- **{}** methods analyzed\n\n", call_chain_result.stats.total_methods));

                content.push_str("### System Understanding\n\n");
                content.push_str(&call_chain_result.system_synthesis.overall_description);
                content.push_str("\n\n");

                if !call_chain_result.system_synthesis.key_themes.is_empty() {
                    content.push_str("**Key Architectural Themes**:\n");
                    for theme in &call_chain_result.system_synthesis.key_themes {
                        content.push_str(&format!("- {}\n", theme));
                    }
                    content.push_str("\n");
                }

                // Entry points section
                content.push_str("## System Entry Points\n\n");
                content.push_str("These are the main ways users and external systems interact with this codebase:\n\n");

                for entry_point in &call_chain_result.entry_points {
                    content.push_str(&format!(
                        "### {} ({:?})\n\n",
                        entry_point.signature.display_name(),
                        entry_point.entry_type
                    ));
                    content.push_str(&format!("**File**: {}\n", entry_point.signature.file_path.display()));
                    content.push_str(&format!("**Confidence**: {:.2}\n", entry_point.confidence));
                    content.push_str(&format!("**Analysis**: {}\n\n", entry_point.reasoning));
                }

                // Detailed analysis links
                content.push_str("## Detailed Analysis\n\n");
                content.push_str("For detailed workflow analysis, see:\n\n");
                content.push_str("- [Call Chain Groups](./groups/) - Detailed execution path analysis\n");
                content.push_str("- [Package Documentation](./packages/) - Enhanced package-level documentation\n");
                content.push_str("- [Call Graph Data](./call_graph.json) - Complete call graph for visualization\n\n");

                content.push_str("---\n\n*This system overview was generated by Codesworth's call-chain analysis engine.*\n");

                // Preserve existing edits if file exists
                let final_content = if overview_path.exists() && self.config.generation.preserve_edits {
                    let existing_content = std::fs::read_to_string(&overview_path)?;
                    self.protector.merge_with_existing(&content, &existing_content)
                        .map_err(|e| anyhow::anyhow!("Failed to merge system overview: {}", e))?
                } else {
                    content
                };

                std::fs::write(&overview_path, final_content)?;
                info!("✅ Call-chain system overview generated: {}", overview_path.display());
            }
        }

        Ok(())
    }

    // Helper methods for call-chain integration

    fn extract_package_name_from_path(&self, file_path: &PathBuf) -> Option<String> {
        let path_str = file_path.to_string_lossy();

        // Try to extract package name using similar logic as PackageAnalyzer
        if let Some(src_index) = path_str.find("/src/") {
            let after_src = &path_str[src_index + 5..];
            if let Some(first_slash) = after_src.find('/') {
                let package = &after_src[..first_slash];
                if package != "bin" && package != "test" && package != "tests" {
                    return Some(package.to_string());
                }
            }
        }

        // Fallback to parent directory name
        file_path.parent()
            .and_then(|p| p.file_name())
            .and_then(|name| name.to_str())
            .map(|s| s.to_string())
    }

    fn build_package_overview_prompt(
        &self,
        package_name: &str,
        groups: &[&super::CallChainGroup],
        analyses: &[super::GroupAnalysis],
        human_context: &HumanContext,
    ) -> String {
        let mut prompt = String::new();

        prompt.push_str(&format!(
            "Generate comprehensive documentation for the '{}' package based on call-chain analysis.\n\n",
            package_name
        ));

        prompt.push_str("CALL CHAIN ANALYSIS RESULTS:\n");
        for (i, group) in groups.iter().enumerate() {
            if let Some(analysis) = analyses.get(i) {
                prompt.push_str(&format!(
                    "Group {}: {} (confidence: {:.2})\n",
                    i + 1, analysis.description, analysis.confidence
                ));
                prompt.push_str(&format!("  - {} call chains\n", group.call_chains.len()));
                prompt.push_str(&format!("  - {} files involved\n", group.involved_files.len()));
                prompt.push_str(&format!("  - Complexity: {}\n", group.total_complexity));
            }
        }

        if let Some(readme) = &human_context.readme_content {
            prompt.push_str("\nPROJECT CONTEXT:\n");
            prompt.push_str(readme);
            prompt.push_str("\n");
        }

        prompt.push_str("\nGenerate documentation that explains:\n");
        prompt.push_str("1. What this package does in the business context\n");
        prompt.push_str("2. Its role in the overall system architecture\n");
        prompt.push_str("3. Key workflows it participates in\n");
        prompt.push_str("4. How it integrates with other system components\n");
        prompt.push_str("5. Important implementation considerations\n\n");
        prompt.push_str("Focus on helping developers understand the package's purpose and how to work with it effectively.\n");

        prompt
    }

    fn build_system_overview_prompt(&self, call_chain_result: &CallChainAnalysisResult, human_context: &HumanContext) -> String {
        let mut prompt = String::new();

        prompt.push_str("Generate a comprehensive system overview based on call-chain analysis.\n\n");

        prompt.push_str("SYSTEM ANALYSIS RESULTS:\n");
        prompt.push_str(&format!("System Purpose: {}\n", call_chain_result.system_synthesis.overall_description));
        prompt.push_str(&format!("Key Themes: {}\n", call_chain_result.system_synthesis.key_themes.join(", ")));
        prompt.push_str(&format!("Entry Points: {}\n", call_chain_result.entry_points.len()));
        prompt.push_str(&format!("Execution Groups: {}\n", call_chain_result.call_chain_groups.len()));

        prompt.push_str("\nENTRY POINTS:\n");
        for ep in &call_chain_result.entry_points {
            prompt.push_str(&format!("- {} ({:?}): {}\n",
                                     ep.signature.display_name(), ep.entry_type, ep.reasoning));
        }

        if let Some(readme) = &human_context.readme_content {
            prompt.push_str("\nEXISTING DOCUMENTATION:\n");
            prompt.push_str(readme);
        }

        prompt.push_str("\nGenerate a system overview that:\n");
        prompt.push_str("1. Clearly explains what this system does from a business perspective\n");
        prompt.push_str("2. Describes the main user workflows and entry points\n");
        prompt.push_str("3. Explains the architectural approach and design decisions\n");
        prompt.push_str("4. Provides context for how the different parts work together\n");
        prompt.push_str("5. Gives developers what they need to understand and contribute to the system\n\n");
        prompt.push_str("Be specific about business value and user workflows, not just technical implementation.\n");

        prompt
    }

    fn build_minimal_context_for_package(&self, package_name: &str) -> Result<super::DocumentationContext> {
        use std::path::PathBuf;

        Ok(super::DocumentationContext {
            file: super::ParsedFile {
                path: PathBuf::from(format!("packages/{}", package_name)),
                language: "multi".to_string(),
                content_hash: "package".to_string(),
                modified_time: std::time::SystemTime::now(),
                modules: vec![],
                file_docs: None,
                source_content: "".to_string(),
            },
            target_module: None,
            related_files: vec![],
            project_info: super::ProjectInfo {
                name: format!("Package: {}", package_name),
                description: None,
                language: "multi".to_string(),
                project_type: Some("package".to_string()),
            },
            architecture_docs: None,
        })
    }

    fn build_minimal_context_for_system(&self) -> Result<super::DocumentationContext> {
        use std::path::PathBuf;

        Ok(super::DocumentationContext {
            file: super::ParsedFile {
                path: PathBuf::from("system"),
                language: "system".to_string(),
                content_hash: "system".to_string(),
                modified_time: std::time::SystemTime::now(),
                modules: vec![],
                file_docs: None,
                source_content: "".to_string(),
            },
            target_module: None,
            related_files: vec![],
            project_info: super::ProjectInfo {
                name: "System Overview".to_string(),
                description: None,
                language: "multi".to_string(),
                project_type: Some("system".to_string()),
            },
            architecture_docs: None,
        })
    }

    // Implement the CLI interface methods (keeping existing functionality)

    pub async fn init(&self, path: Option<PathBuf>, non_interactive: bool) -> Result<()> {
        let target_dir = path.unwrap_or_else(|| std::env::current_dir().unwrap());
        info!("Initializing Codesworth in: {}", target_dir.display());
        Ok(())
    }

    pub async fn sync(&self, dry_run: bool, fail_on_changes: bool) -> Result<()> {
        info!("🔄 Synchronizing documentation with call-chain analysis...");

        if dry_run {
            info!("📋 Dry run mode - showing what would be updated");
        }

        // TODO: Implement sync with call-chain approach
        // This would re-run call-chain analysis and update only changed documentation
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

    // Remove the problematic method
    // Call chain grouper methods are accessed directly when needed
}