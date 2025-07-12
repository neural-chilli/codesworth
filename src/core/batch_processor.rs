// src/core/batch_processor.rs
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

use crate::error::Result;
use super::{
    LlmDocumenter, DocumentationContext, EnhancementRequest, EnhancementResponse,
    EnhancementType, ProjectInfo, ArchitectureDocs,
    package_analysis::PackageAnalysis
};

/// Request for batch documentation generation of a package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchDocumentationRequest {
    /// The package being analyzed
    pub package_analysis: PackageAnalysis,

    /// Human-authored context found in the project
    pub human_context: HumanContext,

    /// System-wide context for better understanding
    pub system_context: SystemContext,

    /// What aspects to focus the enhancement on
    pub enhancement_focus: AnalysisFocus,
}

/// Human-authored documentation and context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanContext {
    /// README content from project root
    pub readme_content: Option<String>,

    /// Architecture documents found in docs/
    pub architecture_docs: Vec<ArchitectureDoc>,

    /// ADRs (Architectural Decision Records)
    pub adrs: Vec<ArchitecturalDecision>,

    /// Significant inline comments with architectural insights
    pub inline_comments: Vec<ArchitecturalComment>,

    /// Configuration files that reveal system behavior
    pub configuration_hints: Vec<ConfigurationHint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureDoc {
    pub title: String,
    pub content: String,
    pub file_path: String,
    pub relevance_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitecturalDecision {
    pub title: String,
    pub status: String,  // Proposed, Accepted, Deprecated, etc.
    pub context: String,
    pub decision: String,
    pub consequences: String,
    pub date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitecturalComment {
    pub content: String,
    pub file_path: String,
    pub line_number: usize,
    pub category: CommentCategory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommentCategory {
    DesignDecision,    // Why this approach was chosen
    PerformanceNote,   // Performance implications
    SecurityNote,      // Security considerations
    IntegrationNote,   // How it integrates with other systems
    TechnicalDebt,     // Known limitations or TODO items
    BusinessLogic,     // Domain-specific business rules
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurationHint {
    pub source: String,      // Cargo.toml, package.json, etc.
    pub category: String,    // dependencies, scripts, etc.
    pub insights: Vec<String>, // What this reveals about the system
}

/// System-wide context spanning multiple packages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemContext {
    /// Related packages that this one interacts with
    pub related_packages: Vec<RelatedPackage>,

    /// Common patterns across the codebase
    pub common_patterns: Vec<PatternUsage>,

    /// System-wide architectural themes
    pub architectural_themes: Vec<ArchitecturalTheme>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedPackage {
    pub name: String,
    pub relationship: String,  // "depends on", "provides interface to", etc.
    pub interaction_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternUsage {
    pub pattern_name: String,  // Repository, Factory, Observer, etc.
    pub usage_context: String,
    pub benefits: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitecturalTheme {
    pub theme: String,         // Event Sourcing, CQRS, Microservices, etc.
    pub manifestation: String, // How it's implemented in this system
    pub rationale: String,     // Why this approach was chosen
}

/// What aspects to focus the analysis on
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisFocus {
    pub focus_areas: Vec<FocusArea>,
    pub depth_level: DepthLevel,
    pub target_audience: TargetAudience,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FocusArea {
    Purpose,           // What does this package do?
    Architecture,      // How is it structured?
    Integrations,      // How does it connect to other parts?
    Performance,       // Performance characteristics
    Security,          // Security considerations
    Maintenance,       // How to modify/extend it
    Troubleshooting,   // Common issues and solutions
    Testing,           // How to test it
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DepthLevel {
    Overview,          // High-level summary
    Detailed,          // Comprehensive documentation
    Reference,         // Complete API reference
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TargetAudience {
    NewTeamMember,     // Someone joining the team
    ExperiencedDev,    // Experienced developer unfamiliar with this codebase
    Maintainer,        // Someone who will modify this code
    Integrator,        // Someone who will use this as a dependency
}

/// Response from batch LLM processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchDocumentationResponse {
    /// Generated package overview
    pub package_overview: String,

    /// Key insights discovered during analysis
    pub key_insights: Vec<KeyInsight>,

    /// API documentation sections
    pub api_documentation: HashMap<String, String>,

    /// Integration guidance
    pub integration_guide: Option<String>,

    /// Maintenance notes
    pub maintenance_notes: Option<String>,

    /// Cross-references to related packages
    pub cross_references: Vec<CrossReference>,

    /// Confidence and quality indicators
    pub metadata: BatchResponseMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyInsight {
    pub category: InsightCategory,
    pub title: String,
    pub description: String,
    pub importance: InsightImportance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InsightCategory {
    ArchitecturalDecision,
    PerformanceImplication,
    SecurityConsideration,
    IntegrationPattern,
    MaintenanceGotcha,
    BusinessLogic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InsightImportance {
    Critical,    // Must understand to work with this code
    Important,   // Should understand for effective work
    Useful,      // Good to know for optimization
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossReference {
    pub target_package: String,
    pub relationship: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResponseMetadata {
    pub confidence_score: f32,
    pub analysis_completeness: f32,
    pub context_utilization: f32,
    pub suggestions_for_improvement: Vec<String>,
}

/// Batch processor that efficiently processes packages using LLM
pub struct BatchProcessor;

impl BatchProcessor {
    pub fn new() -> Self {
        Self
    }

    /// Process a package with comprehensive context for high-quality documentation
    pub async fn process_package(
        &self,
        request: BatchDocumentationRequest,
        llm_documenter: &dyn LlmDocumenter,
    ) -> Result<BatchDocumentationResponse> {
        // Build comprehensive prompt with deep semantic analysis
        let prompt = self.build_semantic_analysis_prompt(&request)?;

        // Make single LLM call with rich context
        let enhancement_request = EnhancementRequest {
            enhancement_type: EnhancementType::Custom(prompt),
            context: self.build_documentation_context(&request)?,
            current_content: None,
            focus_areas: request.enhancement_focus.focus_areas.iter()
                .map(|area| format!("{:?}", area).to_lowercase())
                .collect(),
        };

        let llm_response = llm_documenter.enhance_documentation(enhancement_request).await?;

        // Parse and structure the LLM response
        self.parse_llm_response(llm_response, &request).await
    }

    fn build_semantic_analysis_prompt(&self, request: &BatchDocumentationRequest) -> Result<String> {
        let pkg = &request.package_analysis;
        let mut prompt = String::new();

        // System instruction with focus on semantic understanding
        prompt.push_str(&format!(
            "You are analyzing the '{}' package to understand its ACTUAL business purpose and domain.\n\
            Your goal is to understand what this code REALLY does in the business context,\n\
            not just describe its technical implementation.\n\n",
            pkg.package_name
        ));

        // Extract and analyze semantic patterns
        let semantic_analysis = self.extract_semantic_intelligence(pkg);

        prompt.push_str("SEMANTIC CODE ANALYSIS:\n");
        prompt.push_str(&format!("Package: {} ({})\n", pkg.package_name, pkg.generate_summary()));
        prompt.push_str(&format!("Complexity: {:.2} | Files: {} | API Elements: {}\n\n",
                                 pkg.complexity_score(),
                                 pkg.files.len(),
                                 pkg.public_api.functions.len() + pkg.public_api.types.len()
        ));

        // Entity relationship analysis
        if !semantic_analysis.entity_relationships.is_empty() {
            prompt.push_str("ENTITY RELATIONSHIPS:\n");
            for relationship in &semantic_analysis.entity_relationships {
                prompt.push_str(&format!("- {}\n", relationship));
            }
            prompt.push_str("\n");
        }

        // Business logic patterns
        if !semantic_analysis.business_logic_patterns.is_empty() {
            prompt.push_str("BUSINESS LOGIC PATTERNS:\n");
            for pattern in &semantic_analysis.business_logic_patterns {
                prompt.push_str(&format!("- {}\n", pattern));
            }
            prompt.push_str("\n");
        }

        // Data flow analysis
        if !semantic_analysis.data_flows.is_empty() {
            prompt.push_str("DATA FLOW ANALYSIS:\n");
            for flow in &semantic_analysis.data_flows {
                prompt.push_str(&format!("- {}\n", flow));
            }
            prompt.push_str("\n");
        }

        // Domain vocabulary
        if !semantic_analysis.domain_vocabulary.is_empty() {
            prompt.push_str("DOMAIN VOCABULARY:\n");
            for (term, context) in &semantic_analysis.domain_vocabulary {
                prompt.push_str(&format!("- {}: {}\n", term, context));
            }
            prompt.push_str("\n");
        }

        // Configuration analysis
        if !semantic_analysis.configuration_insights.is_empty() {
            prompt.push_str("CONFIGURATION INSIGHTS:\n");
            for insight in &semantic_analysis.configuration_insights {
                prompt.push_str(&format!("- {}\n", insight));
            }
            prompt.push_str("\n");
        }

        // Human context integration (without interpretation)
        if let Some(readme) = &request.human_context.readme_content {
            prompt.push_str("PROJECT CONTEXT (from README):\n");
            prompt.push_str(readme);
            prompt.push_str("\n\n");
        }

        if !request.human_context.architecture_docs.is_empty() {
            prompt.push_str("ARCHITECTURAL DOCUMENTATION:\n");
            for doc in &request.human_context.architecture_docs {
                prompt.push_str(&format!("## {}\n{}\n\n", doc.title, doc.content));
            }
        }

        // Code structure and API surface
        prompt.push_str("PUBLIC API SURFACE:\n");
        if !pkg.public_api.entry_points.is_empty() {
            prompt.push_str("Entry Points:\n");
            for entry_point in &pkg.public_api.entry_points {
                prompt.push_str(&format!(
                    "- {} ({:?}): {}\n",
                    entry_point.name,
                    entry_point.entry_type,
                    entry_point.description.as_deref().unwrap_or("No description")
                ));
            }
            prompt.push_str("\n");
        }

        if !pkg.public_api.types.is_empty() {
            prompt.push_str("Public Types:\n");
            for api_type in &pkg.public_api.types {
                prompt.push_str(&format!(
                    "- {} ({}): {}\n",
                    api_type.name,
                    api_type.type_kind,
                    api_type.docs.as_deref().unwrap_or("No documentation")
                ));
            }
            prompt.push_str("\n");
        }

        if !pkg.public_api.functions.is_empty() {
            prompt.push_str("Public Functions:\n");
            for function in &pkg.public_api.functions {
                prompt.push_str(&format!(
                    "- {}: {}\n",
                    function.signature,
                    function.docs.as_deref().unwrap_or("No documentation")
                ));
            }
            prompt.push_str("\n");
        }

        // Task instruction with semantic focus
        prompt.push_str(&format!(
            "ANALYSIS TASK:\n\
            Based on the semantic analysis above, determine what this '{}' package ACTUALLY does:\n\n\
            1. BUSINESS DOMAIN ANALYSIS:\n\
               - What real-world business problem does this solve?\n\
               - What industry/domain does this serve?\n\
               - What are the core business concepts and entities?\n\
               - Who are the actual users and what do they accomplish?\n\n\
            2. BUSINESS LOGIC ANALYSIS:\n\
               - What are the key business rules and workflows?\n\
               - What domain-specific decisions or algorithms exist?\n\
               - What business constraints or invariants are enforced?\n\
               - What edge cases or business exceptions are handled?\n\n\
            3. PRACTICAL PURPOSE:\n\
               - How do users actually interact with this system?\n\
               - What are the common usage patterns and workflows?\n\
               - What business value does this package provide?\n\
               - How does this fit into the larger business context?\n\n\
            CRITICAL INSTRUCTIONS:\n\
            - Focus on BUSINESS SEMANTICS, not just technical patterns\n\
            - Look for the ACTUAL DOMAIN this serves (entertainment, finance, etc.)\n\
            - Identify SPECIFIC use cases, not generic \"communication\" or \"processing\"\n\
            - Connect entity relationships to business concepts\n\
            - Explain WHY this system exists from a business perspective\n\
            - Avoid generic tech buzzwords - be specific about the business domain\n\n\
            Target audience: {}\n\
            Depth level: {:?}\n",
            pkg.package_name,
            self.describe_target_audience(&request.enhancement_focus.target_audience),
            request.enhancement_focus.depth_level
        ));

        Ok(prompt)
    }

    /// Extract semantic intelligence from package analysis
    fn extract_semantic_intelligence(&self, pkg: &PackageAnalysis) -> SemanticIntelligence {
        let mut intelligence = SemanticIntelligence::new();

        // Analyze all source content for semantic patterns
        let combined_content = pkg.files.iter()
            .map(|f| &f.source_content)
            .cloned()
            .collect::<Vec<_>>()
            .join(" ");

        // Extract entity relationships
        intelligence.entity_relationships = self.analyze_entity_relationships(pkg);

        // Extract business logic patterns
        intelligence.business_logic_patterns = self.analyze_business_logic_patterns(&combined_content);

        // Extract data flow patterns
        intelligence.data_flows = self.analyze_data_flows(&combined_content);

        // Extract domain vocabulary
        intelligence.domain_vocabulary = self.extract_domain_vocabulary(&combined_content);

        // Extract configuration insights
        intelligence.configuration_insights = self.analyze_configuration_context(pkg);

        intelligence
    }

    /// Analyze entity relationships in the code
    fn analyze_entity_relationships(&self, pkg: &PackageAnalysis) -> Vec<String> {
        let mut relationships = Vec::new();

        // Look for class/type relationships
        for api_type in &pkg.public_api.types {
            // Skip technical types
            if self.is_technical_type(&api_type.name) {
                continue;
            }

            // Analyze type relationships
            if api_type.type_kind == "class" || api_type.type_kind == "struct" {
                relationships.push(format!(
                    "{} entity with {} methods ({})",
                    api_type.name,
                    api_type.methods.len(),
                    api_type.docs.as_deref().unwrap_or("business entity")
                ));
            }

            // Look for response/request patterns
            if api_type.name.contains("Response") || api_type.name.contains("Request") {
                let base_name = api_type.name.replace("Response", "").replace("Request", "");
                relationships.push(format!(
                    "{} data transfer for {} operations",
                    api_type.name, base_name
                ));
            }
        }

        // Look for service relationships
        for function in &pkg.public_api.functions {
            if function.name.contains("process") || function.name.contains("handle") {
                relationships.push(format!(
                    "{} processes business operations",
                    function.name
                ));
            }
        }

        relationships
    }

    /// Analyze business logic patterns
    fn analyze_business_logic_patterns(&self, content: &str) -> Vec<String> {
        let mut patterns = Vec::new();

        // Look for mapping/lookup patterns with specific examples
        if content.contains("put(") && content.contains("get(") {
            let mapping_examples = self.extract_mapping_examples(content);
            if !mapping_examples.is_empty() {
                patterns.push(format!("Key-value mapping system: {}", mapping_examples.join(", ")));
            }
        }

        // Look for probability/percentage logic
        if let Some(probability_details) = self.extract_probability_details(content) {
            patterns.push(probability_details);
        }

        // Look for persona/character patterns
        if content.to_lowercase().contains("persona") || content.to_lowercase().contains("character") {
            patterns.push("Character/persona-based behavior system".to_string());
        }

        // Look for response generation patterns
        if content.contains("response") && content.contains("generate") {
            patterns.push("Dynamic response generation based on input".to_string());
        }

        // Look for random selection patterns
        if content.contains("random") && content.contains("size()") {
            patterns.push("Random selection from available options".to_string());
        }

        // Look for alias/shortcut patterns
        if content.contains("alias") {
            patterns.push("Alias/shortcut mapping for user convenience".to_string());
        }

        // Look for validation patterns
        if content.contains("validate") || content.contains("check") {
            patterns.push("Input validation and business rule enforcement".to_string());
        }

        patterns
    }

    /// Analyze data flows in the system
    fn analyze_data_flows(&self, content: &str) -> Vec<String> {
        let mut flows = Vec::new();

        // WebSocket flow patterns
        if content.contains("WebSocket") || content.contains("@MessageMapping") {
            flows.push("Real-time bidirectional communication via WebSocket".to_string());
        }

        // HTTP endpoint patterns
        if content.contains("@GetMapping") || content.contains("@PostMapping") {
            flows.push("HTTP request/response processing".to_string());
        }

        // Message processing flows
        if content.contains("Message") && content.contains("process") {
            flows.push("Message processing and routing workflow".to_string());
        }

        // Event-driven patterns
        if content.contains("event") || content.contains("trigger") {
            flows.push("Event-driven processing and triggering".to_string());
        }

        flows
    }

    /// Extract domain-specific vocabulary with context
    fn extract_domain_vocabulary(&self, content: &str) -> Vec<(String, String)> {
        let mut vocabulary = Vec::new();

        // Extract meaningful words with frequency
        let words: Vec<&str> = content.split_whitespace()
            .filter(|word| word.len() > 3)
            .filter(|word| !self.is_technical_noise(word))
            .collect();

        let mut word_counts = std::collections::HashMap::new();
        for word in words {
            let clean_word = word.to_lowercase()
                .trim_matches(|c: char| !c.is_alphabetic())
                .to_string();
            if clean_word.len() > 3 && self.seems_domain_specific(&clean_word) {
                *word_counts.entry(clean_word).or_insert(0) += 1;
            }
        }

        // Extract terms that appear frequently
        for (word, count) in word_counts {
            if count >= 2 {
                let context = self.extract_word_context(&word, content);
                vocabulary.push((word, context));
            }
        }

        vocabulary.sort_by(|a, b| b.1.len().cmp(&a.1.len())); // Sort by context richness
        vocabulary.truncate(10); // Keep most relevant terms
        vocabulary
    }

    /// Analyze configuration for business context
    fn analyze_configuration_context(&self, pkg: &PackageAnalysis) -> Vec<String> {
        let mut insights = Vec::new();

        // Look for specific dependencies that reveal business context
        for dep in &pkg.dependencies.external_deps {
            match dep.name.to_lowercase().as_str() {
                name if name.contains("websocket") => {
                    insights.push("Real-time communication capabilities".to_string());
                }
                name if name.contains("message") => {
                    insights.push("Message processing and routing".to_string());
                }
                name if name.contains("spring") => {
                    insights.push("Enterprise web application framework".to_string());
                }
                name if name.contains("lombok") => {
                    insights.push("Java development with reduced boilerplate".to_string());
                }
                _ => {}
            }
        }

        insights
    }

    // Helper methods for semantic analysis

    fn extract_mapping_examples(&self, content: &str) -> Vec<String> {
        let mut examples = Vec::new();

        // Look for put() statements with string literals
        let lines: Vec<&str> = content.lines().collect();
        for line in lines {
            if line.contains("put(") && line.contains("\"") {
                if let Some(example) = self.parse_mapping_line(line) {
                    examples.push(example);
                    if examples.len() >= 3 { // Limit examples
                        break;
                    }
                }
            }
        }

        examples
    }

    fn parse_mapping_line(&self, line: &str) -> Option<String> {
        // Simple extraction of put("key", "value") patterns
        if let Ok(re) = regex::Regex::new(r#"put\("([^"]+)",\s*"([^"]+)"\)"#) {
            if let Some(captures) = re.captures(line) {
                let key = captures.get(1)?.as_str();
                let value = captures.get(2)?.as_str();
                return Some(format!("{}→{}", key, value));
            }
        }
        None
    }

    fn extract_probability_details(&self, content: &str) -> Option<String> {
        // Look for probability assignments
        let lines: Vec<&str> = content.lines().collect();
        for line in lines {
            let line_lower = line.to_lowercase();
            if (line_lower.contains("probability") || line_lower.contains("chance")) && line_lower.contains("=") {
                if let Some(prob_value) = self.extract_probability_value(line) {
                    if line_lower.contains("followup") || line_lower.contains("follow") {
                        return Some(format!("Follow-up probability: {} (automatic conversation continuation)", prob_value));
                    } else {
                        return Some(format!("Probability-based logic: {}", prob_value));
                    }
                }
            }
        }
        None
    }

    fn extract_probability_value(&self, line: &str) -> Option<String> {
        if let Ok(re) = regex::Regex::new(r"([0-9]*\.?[0-9]+)") {
            if let Some(captures) = re.captures(line) {
                return Some(captures.get(1)?.as_str().to_string());
            }
        }
        None
    }

    fn extract_word_context(&self, word: &str, content: &str) -> String {
        // Find context around the word
        let lines: Vec<&str> = content.lines().collect();
        for line in lines {
            if line.to_lowercase().contains(word) {
                // Return a cleaned version of the line as context
                let cleaned = line.trim()
                    .replace("public", "")
                    .replace("private", "")
                    .replace("class", "")
                    .replace("//", "")
                    .trim()
                    .to_string();
                if cleaned.len() > word.len() + 5 {
                    return cleaned;
                }
            }
        }
        "domain concept".to_string()
    }

    fn is_technical_type(&self, type_name: &str) -> bool {
        let technical_patterns = [
            "config", "configuration", "settings", "properties",
            "exception", "error", "result", "handler", "processor",
            "builder", "factory", "adapter", "wrapper", "util"
        ];

        let name_lower = type_name.to_lowercase();
        technical_patterns.iter().any(|pattern| name_lower.contains(pattern))
    }

    fn is_technical_noise(&self, word: &str) -> bool {
        let noise_terms = [
            "string", "boolean", "integer", "float", "double", "char", "byte",
            "array", "list", "vector", "map", "hash", "dict", "set",
            "class", "struct", "enum", "trait", "interface",
            "public", "private", "static", "final", "const", "void",
            "import", "include", "using", "namespace", "package",
            "function", "method", "return", "throw", "catch", "finally",
            "spring", "framework", "library", "util", "helper", "lombok"
        ];
        noise_terms.contains(&word.to_lowercase().as_str())
    }

    fn seems_domain_specific(&self, word: &str) -> bool {
        let word_lower = word.to_lowercase();

        // Skip obvious technical terms
        if self.is_technical_noise(&word_lower) {
            return false;
        }

        // Skip very short words
        if word.len() < 4 {
            return false;
        }

        // Skip words that are clearly technical patterns
        if word_lower.contains("impl") || word_lower.contains("test") ||
            word_lower.contains("config") || word_lower.contains("debug") {
            return false;
        }

        true
    }

    fn build_documentation_context(&self, request: &BatchDocumentationRequest) -> Result<DocumentationContext> {
        let pkg = &request.package_analysis;

        // Use first file as representative (for the interface)
        let representative_file = pkg.files.first()
            .ok_or_else(|| crate::error::CodesworthError::Parser("No files in package".to_string()))?;

        let project_info = ProjectInfo {
            name: request.human_context.readme_content
                .as_ref()
                .and_then(|readme| self.extract_project_name(readme))
                .unwrap_or_else(|| "Unknown Project".to_string()),
            description: request.human_context.readme_content.clone(),
            language: representative_file.language.clone(),
            project_type: Some(self.infer_project_type(&request.human_context)),
        };

        let architecture_docs = self.build_architecture_docs(&request.human_context, &request.system_context);

        Ok(DocumentationContext {
            file: representative_file.clone(),
            target_module: None,
            related_files: pkg.files.clone(),
            project_info,
            architecture_docs: Some(architecture_docs),
        })
    }

    fn build_architecture_docs(&self, human_context: &HumanContext, system_context: &SystemContext) -> ArchitectureDocs {
        let system_overview = human_context.readme_content.clone()
            .or_else(|| {
                human_context.architecture_docs.first()
                    .map(|doc| doc.content.clone())
            });

        let architectural_decisions = human_context.adrs.iter()
            .map(|adr| format!("{}: {}", adr.title, adr.decision))
            .collect();

        let technology_stack = human_context.configuration_hints.iter()
            .flat_map(|hint| hint.insights.clone())
            .collect();

        let design_patterns = system_context.common_patterns.iter()
            .map(|pattern| format!("{}: {}", pattern.pattern_name, pattern.usage_context))
            .collect();

        let integrations = system_context.related_packages.iter()
            .map(|pkg| format!("{}: {}", pkg.name, pkg.interaction_summary))
            .collect();

        ArchitectureDocs {
            system_overview,
            architectural_decisions,
            technology_stack,
            design_patterns,
            integrations,
        }
    }

    async fn parse_llm_response(
        &self,
        llm_response: EnhancementResponse,
        request: &BatchDocumentationRequest,
    ) -> Result<BatchDocumentationResponse> {
        // For now, return the content as package overview
        // In a full implementation, we'd parse structured sections

        let key_insights = self.extract_key_insights(&llm_response.content, request);
        let cross_references = self.extract_cross_references(&llm_response.content, request);

        Ok(BatchDocumentationResponse {
            package_overview: llm_response.content,
            key_insights,
            api_documentation: HashMap::new(), // TODO: Parse API sections
            integration_guide: None, // TODO: Extract integration section
            maintenance_notes: None, // TODO: Extract maintenance section
            cross_references,
            metadata: BatchResponseMetadata {
                confidence_score: llm_response.confidence.unwrap_or(0.8),
                analysis_completeness: 0.9, // TODO: Calculate based on coverage
                context_utilization: 0.85, // TODO: Calculate based on context usage
                suggestions_for_improvement: llm_response.suggestions,
            },
        })
    }

    // Helper methods

    fn describe_target_audience(&self, audience: &TargetAudience) -> String {
        match audience {
            TargetAudience::NewTeamMember => "A new team member who needs to understand this codebase",
            TargetAudience::ExperiencedDev => "An experienced developer unfamiliar with this specific codebase",
            TargetAudience::Maintainer => "A developer who will be modifying and maintaining this code",
            TargetAudience::Integrator => "A developer who will use this package as a dependency",
        }.to_string()
    }

    fn extract_project_name(&self, readme: &str) -> Option<String> {
        // Try to extract project name from first heading
        readme.lines()
            .find(|line| line.starts_with("# "))
            .map(|line| line.trim_start_matches("# ").trim().to_string())
    }

    fn infer_project_type(&self, human_context: &HumanContext) -> String {
        if human_context.configuration_hints.iter()
            .any(|hint| hint.source.contains("pom.xml") && hint.insights.iter().any(|i| i.contains("application"))) {
            "web application".to_string()
        } else if human_context.configuration_hints.iter()
            .any(|hint| hint.insights.iter().any(|i| i.contains("web") || i.contains("server"))) {
            "web service".to_string()
        } else {
            "application".to_string()
        }
    }

    fn extract_key_insights(&self, content: &str, _request: &BatchDocumentationRequest) -> Vec<KeyInsight> {
        // TODO: Parse structured insights from LLM response
        // For now, return empty vec
        vec![]
    }

    fn extract_cross_references(&self, _content: &str, request: &BatchDocumentationRequest) -> Vec<CrossReference> {
        // Generate cross-references from system context
        request.system_context.related_packages.iter()
            .map(|pkg| CrossReference {
                target_package: pkg.name.clone(),
                relationship: pkg.relationship.clone(),
                description: pkg.interaction_summary.clone(),
            })
            .collect()
    }
}

/// Semantic intelligence extracted from code analysis
#[derive(Debug, Clone)]
struct SemanticIntelligence {
    /// Entity relationships found in the code
    pub entity_relationships: Vec<String>,

    /// Business logic patterns identified
    pub business_logic_patterns: Vec<String>,

    /// Data flow patterns
    pub data_flows: Vec<String>,

    /// Domain-specific vocabulary with context
    pub domain_vocabulary: Vec<(String, String)>,

    /// Configuration insights
    pub configuration_insights: Vec<String>,
}

impl SemanticIntelligence {
    fn new() -> Self {
        Self {
            entity_relationships: Vec::new(),
            business_logic_patterns: Vec::new(),
            data_flows: Vec::new(),
            domain_vocabulary: Vec::new(),
            configuration_insights: Vec::new(),
        }
    }
}

impl Default for BatchProcessor {
    fn default() -> Self {
        Self::new()
    }
}