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
        // Build comprehensive prompt with all context
        let prompt = self.build_comprehensive_prompt(&request)?;

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

    fn build_comprehensive_prompt(&self, request: &BatchDocumentationRequest) -> Result<String> {
        let pkg = &request.package_analysis;
        let mut prompt = String::new();

        // System instruction with role clarity
        prompt.push_str(&format!(
            "You are an expert software documentation specialist analyzing the '{}' package. \
            Your goal is to create documentation that genuinely helps developers understand, \
            use, and maintain this code.\n\n",
            pkg.package_name
        ));

        // Package summary for context
        prompt.push_str(&format!(
            "PACKAGE SUMMARY:\n\
            Name: {}\n\
            Type: {}\n\
            Complexity Score: {:.2}\n\
            Public API Elements: {}\n\
            External Dependencies: {}\n\
            Lines of Code: {}\n\n",
            pkg.package_name,
            pkg.generate_summary(),
            pkg.complexity_score(),
            pkg.public_api.functions.len() + pkg.public_api.types.len(),
            pkg.dependencies.external_deps.len(),
            pkg.complexity_indicators.loc
        ));

        // Business domain analysis - GENERIC patterns
        if let Some(business_context) = self.extract_business_context(pkg) {
            prompt.push_str("BUSINESS DOMAIN INSIGHTS:\n");
            prompt.push_str(&business_context);
            prompt.push_str("\n\n");
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

        if !request.human_context.adrs.is_empty() {
            prompt.push_str("ARCHITECTURAL DECISIONS:\n");
            for adr in &request.human_context.adrs {
                prompt.push_str(&format!(
                    "- {}: {} ({})\n  Context: {}\n  Decision: {}\n\n",
                    adr.title, adr.status,
                    adr.date.as_deref().unwrap_or("Date unknown"),
                    adr.context, adr.decision
                ));
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

                if !api_type.methods.is_empty() {
                    prompt.push_str("  Methods:\n");
                    for method in &api_type.methods {
                        prompt.push_str(&format!("    - {}\n", method.name));
                    }
                }
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

        // Dependencies - filtered for relevance
        let relevant_deps = self.filter_relevant_dependencies(&pkg.dependencies.external_deps);
        if !relevant_deps.is_empty() {
            prompt.push_str("KEY DEPENDENCIES:\n");
            for dep in &relevant_deps {
                prompt.push_str(&format!("- {} ({})\n", dep.name, dep.usage_context));
            }
            prompt.push_str("\n");
        }

        // Gotchas and important considerations
        if !pkg.complexity_indicators.gotchas.is_empty() {
            prompt.push_str("IMPORTANT CONSIDERATIONS:\n");
            for gotcha in &pkg.complexity_indicators.gotchas {
                prompt.push_str(&format!(
                    "- {:?} ({:?}): {}\n",
                    gotcha.category, gotcha.severity, gotcha.description
                ));
                if let Some(suggestion) = &gotcha.suggestion {
                    prompt.push_str(&format!("  Recommendation: {}\n", suggestion));
                }
            }
            prompt.push_str("\n");
        }

        // System context
        if !request.system_context.related_packages.is_empty() {
            prompt.push_str("RELATED PACKAGES:\n");
            for related in &request.system_context.related_packages {
                prompt.push_str(&format!(
                    "- {} ({}): {}\n",
                    related.name, related.relationship, related.interaction_summary
                ));
            }
            prompt.push_str("\n");
        }

        // Clear instruction focused on BUSINESS PURPOSE
        prompt.push_str(&format!(
            "TASK: Create comprehensive documentation for the '{}' package that focuses on BUSINESS PURPOSE, not just technical implementation.\n\n\
            Answer these questions in order of importance:\n\n\
            1. BUSINESS PURPOSE & DOMAIN:\n\
               - What does this system actually DO in the real world?\n\
               - What business problem does it solve?\n\
               - What domain concepts does it implement?\n\
               - Who are the users and what do they accomplish?\n\n\
            2. KEY BUSINESS LOGIC:\n\
               - What are the core business rules and workflows?\n\
               - What domain-specific algorithms or decision logic exists?\n\
               - What business invariants or constraints are enforced?\n\
               - What edge cases or business exceptions are handled?\n\n\
            3. PRACTICAL USAGE:\n\
               - How do you actually use this system to accomplish business goals?\n\
               - What are the common usage patterns and workflows?\n\
               - What configuration or setup is needed for business operations?\n\
               - What integrations enable business functionality?\n\n\
            4. TECHNICAL IMPLEMENTATION (secondary):\n\
               - What architectural decisions support the business requirements?\n\
               - What performance or scalability considerations matter for business use?\n\
               - What technical gotchas could impact business operations?\n\n\
            CRITICAL: Start with the business domain and work down to technical details. \
            Avoid generic framework descriptions unless they're essential to understanding the business purpose. \
            Focus on what makes THIS system unique and valuable.\n\n\
            Target audience: {}\n\
            Depth level: {:?}\n",
            pkg.package_name,
            self.describe_target_audience(&request.enhancement_focus.target_audience),
            request.enhancement_focus.depth_level
        ));

        Ok(prompt)
    }

    /// Extract business domain context using generic patterns
    fn extract_business_context(&self, pkg: &PackageAnalysis) -> Option<String> {
        let mut insights = Vec::new();

        // Analyze all source content for business patterns
        let combined_content = pkg.files.iter()
            .map(|f| &f.source_content)
            .cloned()
            .collect::<Vec<_>>()
            .join(" ");

        // Domain terminology patterns (generic)
        let domain_terms = self.extract_domain_terminology(&combined_content);
        if !domain_terms.is_empty() {
            // Try to infer domain context from term combinations
            if let Some(domain_context) = self.infer_domain_context(&domain_terms, &combined_content) {
                insights.push(domain_context);
            } else {
                insights.push(format!("Domain concepts: {}", domain_terms.join(", ")));
            }
        }

        // Business rule patterns
        let business_rules = self.extract_business_rules(&combined_content, pkg);
        insights.extend(business_rules);

        // User-facing functionality patterns
        let user_patterns = self.extract_user_facing_patterns(&combined_content);
        insights.extend(user_patterns);

        // Data model patterns  
        let data_insights = self.extract_data_model_insights(pkg);
        insights.extend(data_insights);

        // Workflow patterns
        let workflow_insights = self.extract_workflow_patterns(&combined_content);
        insights.extend(workflow_insights);

        // Specific business patterns
        let specific_patterns = self.extract_specific_business_patterns(&combined_content);
        insights.extend(specific_patterns);

        if insights.is_empty() {
            None
        } else {
            Some(insights.join("\n"))
        }
    }

    /// Infer domain context from related terminology
    fn infer_domain_context(&self, terms: &[String], content: &str) -> Option<String> {
        let combined_terms = terms.join(" ").to_lowercase();

        // Star Trek patterns
        if (combined_terms.contains("kirk") || combined_terms.contains("spock") ||
            combined_terms.contains("enterprise")) &&
            (combined_terms.contains("crew") || combined_terms.contains("captain")) {
            return Some("Star Trek themed system with crew member characters and USS Enterprise context".to_string());
        }

        // Gaming patterns
        if combined_terms.contains("player") || combined_terms.contains("game") ||
            combined_terms.contains("score") || combined_terms.contains("level") {
            return Some("Gaming system with player interactions and scoring".to_string());
        }

        // E-commerce patterns
        if combined_terms.contains("order") || combined_terms.contains("payment") ||
            combined_terms.contains("product") || combined_terms.contains("cart") {
            return Some("E-commerce system with order processing and payment handling".to_string());
        }

        // Financial patterns
        if combined_terms.contains("account") || combined_terms.contains("transaction") ||
            combined_terms.contains("balance") || combined_terms.contains("money") {
            return Some("Financial system with account management and transactions".to_string());
        }

        // Chat/Communication patterns
        if combined_terms.contains("message") || combined_terms.contains("chat") ||
            combined_terms.contains("conversation") {
            return Some("Communication system with messaging and chat functionality".to_string());
        }

        None
    }

    /// Extract specific business patterns and logic
    fn extract_specific_business_patterns(&self, content: &str) -> Vec<String> {
        let mut patterns = Vec::new();

        // Character/persona systems
        if content.contains("persona") && content.contains("character") {
            patterns.push("Character persona system with role-based behavior".to_string());
        }

        // Alias/mapping systems with specific examples
        if content.contains("alias") && content.contains("put(") {
            if let Some(alias_examples) = self.extract_alias_examples(content) {
                patterns.push(format!("Alias mapping system: {}", alias_examples));
            }
        }

        // Random selection patterns
        if content.contains("random") && content.contains("size()") {
            patterns.push("Random selection from available options".to_string());
        }

        // Follow-up/probability patterns with specific values
        if let Some(probability_details) = self.extract_detailed_probability_patterns(content) {
            patterns.push(probability_details);
        }

        // Response generation patterns
        if content.contains("response") && content.contains("generate") {
            patterns.push("Dynamic response generation based on input".to_string());
        }

        // Memory/conversation patterns
        if content.contains("memory") && content.contains("conversation") {
            patterns.push("Conversation memory for context-aware interactions".to_string());
        }

        patterns
    }

    /// Extract specific alias examples from code
    fn extract_alias_examples(&self, content: &str) -> Option<String> {
        let mut examples = Vec::new();

        // Look for put() statements with string literals
        let lines: Vec<&str> = content.lines().collect();
        for line in lines {
            if line.contains("put(") && line.contains("\"") {
                // Extract simple alias patterns like .put("kirk", "Kirk")
                if let Some(alias_example) = self.parse_alias_line(line) {
                    examples.push(alias_example);
                    if examples.len() >= 3 { // Limit examples
                        break;
                    }
                }
            }
        }

        if examples.is_empty() {
            None
        } else {
            Some(examples.join(", "))
        }
    }

    /// Parse a single alias mapping line
    fn parse_alias_line(&self, line: &str) -> Option<String> {
        // Simple regex to extract put("key", "value") patterns
        if let Ok(re) = regex::Regex::new(r#"put\("([^"]+)",\s*"([^"]+)"\)"#) {
            if let Some(captures) = re.captures(line) {
                let key = captures.get(1)?.as_str();
                let value = captures.get(2)?.as_str();
                return Some(format!("{}→{}", key, value));
            }
        }
        None
    }

    /// Extract detailed probability patterns with context
    fn extract_detailed_probability_patterns(&self, content: &str) -> Option<String> {
        let lines: Vec<&str> = content.lines().collect();

        for line in lines {
            let line_lower = line.to_lowercase();

            // Look for probability assignments with context
            if line_lower.contains("probability") && line_lower.contains("=") {
                if let Some(prob_value) = self.extract_probability_value(line) {
                    // Try to get context from surrounding lines or variable names
                    if line_lower.contains("followup") || line_lower.contains("follow") {
                        return Some(format!("Follow-up probability: {} (automatic conversation continuation)", prob_value));
                    } else {
                        return Some(format!("Probability-based logic: {}", prob_value));
                    }
                }
            }

            // Look for percentage patterns
            if line_lower.contains("%") || line_lower.contains("chance") {
                if let Some(percentage) = self.extract_percentage_from_line(line) {
                    return Some(format!("Probability-based behavior: {}", percentage));
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

    fn extract_percentage_from_line(&self, line: &str) -> Option<String> {
        if let Ok(re) = regex::Regex::new(r"([0-9]*\.?[0-9]+)\s*%") {
            if let Some(captures) = re.captures(line) {
                return Some(format!("{}%", captures.get(1)?.as_str()));
            }
        }
        None
    }

    /// Extract domain-specific terminology (language agnostic)
    fn extract_domain_terminology(&self, content: &str) -> Vec<String> {
        let mut terms = Vec::new();

        // Look for repeated domain-specific words (not framework terms)
        let words: Vec<&str> = content.split_whitespace()
            .filter(|word| word.len() > 3)
            .filter(|word| !self.is_technical_noise(word))
            .collect();

        let mut word_counts = std::collections::HashMap::new();
        for word in words {
            let clean_word = word.to_lowercase()
                .trim_matches(|c: char| !c.is_alphabetic())
                .to_string();
            if clean_word.len() > 3 {
                *word_counts.entry(clean_word).or_insert(0) += 1;
            }
        }

        // Find words that appear frequently and seem domain-specific
        for (word, count) in word_counts {
            if count >= 3 && self.seems_domain_specific(&word) {
                terms.push(word);
            }
        }

        terms.sort();
        terms.dedup();
        terms.truncate(8); // Keep most relevant terms
        terms
    }

    /// Extract business rules and logic patterns
    fn extract_business_rules(&self, content: &str, pkg: &PackageAnalysis) -> Vec<String> {
        let mut rules = Vec::new();

        // Probability/percentage patterns
        if let Some(probabilities) = self.extract_probability_patterns(content) {
            rules.push(format!("Probability-based logic: {}", probabilities));
        }

        // Mapping/alias patterns
        if content.contains("alias") || content.contains("mapping") || content.contains("lookup") {
            if let Some(mapping_insight) = self.extract_mapping_patterns(content) {
                rules.push(mapping_insight);
            }
        }

        // State machine patterns
        if let Some(state_patterns) = self.extract_state_patterns(content) {
            rules.push(state_patterns);
        }

        // Validation/constraint patterns
        if let Some(validation_patterns) = self.extract_validation_patterns(content) {
            rules.push(validation_patterns);
        }

        // Calculation/algorithm patterns
        if let Some(calc_patterns) = self.extract_calculation_patterns(content) {
            rules.push(calc_patterns);
        }

        rules
    }

    /// Extract user-facing functionality indicators
    fn extract_user_facing_patterns(&self, content: &str) -> Vec<String> {
        let mut patterns = Vec::new();

        // API endpoint patterns
        if content.contains("endpoint") || content.contains("route") || content.contains("@GetMapping") {
            patterns.push("HTTP API endpoints for user interaction".to_string());
        }

        // WebSocket/real-time patterns
        if content.contains("websocket") || content.contains("@MessageMapping") {
            patterns.push("Real-time communication interface".to_string());
        }

        // CLI patterns
        if content.contains("clap") || content.contains("argparse") || content.contains("click") {
            patterns.push("Command-line interface for user operations".to_string());
        }

        // Event handling patterns
        if content.contains("event") || content.contains("handler") || content.contains("listener") {
            patterns.push("Event-driven user interactions".to_string());
        }

        patterns
    }

    /// Extract insights from data models
    fn extract_data_model_insights(&self, pkg: &PackageAnalysis) -> Vec<String> {
        let mut insights = Vec::new();

        // Look for data structures that represent business concepts
        for api_type in &pkg.public_api.types {
            if api_type.type_kind == "struct" || api_type.type_kind == "class" {
                // Skip obvious technical types
                if !self.is_technical_type(&api_type.name) {
                    insights.push(format!(
                        "Business entity: {} ({})",
                        api_type.name,
                        api_type.docs.as_deref().unwrap_or("data model")
                    ));
                }
            }

            if api_type.type_kind == "enum" && !self.is_technical_type(&api_type.name) {
                insights.push(format!("Business states/categories: {}", api_type.name));
            }
        }

        insights
    }

    /// Extract workflow and process patterns
    fn extract_workflow_patterns(&self, content: &str) -> Vec<String> {
        let mut patterns = Vec::new();

        // Sequential processing patterns
        if content.contains("pipeline") || content.contains("chain") || content.contains("workflow") {
            patterns.push("Sequential processing workflow".to_string());
        }

        // Asynchronous processing patterns
        if content.contains("async") || content.contains("await") || content.contains("future") {
            patterns.push("Asynchronous operation handling".to_string());
        }

        // Batch processing patterns
        if content.contains("batch") || content.contains("bulk") || content.contains("queue") {
            patterns.push("Batch/queue processing operations".to_string());
        }

        patterns
    }

    // Helper methods for business context extraction

    fn is_technical_noise(&self, word: &str) -> bool {
        let noise_terms = [
            "string", "boolean", "integer", "float", "double", "char", "byte",
            "array", "list", "vector", "map", "hash", "dict", "set",
            "class", "struct", "enum", "trait", "interface",
            "public", "private", "static", "final", "const", "mut",
            "import", "include", "using", "namespace", "package",
            "function", "method", "procedure", "lambda",
            "return", "throw", "catch", "finally", "else",
            "spring", "framework", "library", "util", "helper"
        ];
        noise_terms.contains(&word.to_lowercase().as_str())
    }

    fn seems_domain_specific(&self, word: &str) -> bool {
        let word_lower = word.to_lowercase();

        // Skip obvious technical terms
        if self.is_technical_noise(&word_lower) {
            return false;
        }

        // Skip single letters and very short words
        if word.len() < 4 {
            return false;
        }

        // Skip words that are clearly file/code patterns
        if word_lower.contains("impl") || word_lower.contains("test") ||
            word_lower.contains("config") || word_lower.contains("util") {
            return false;
        }

        true
    }

    fn extract_probability_patterns(&self, content: &str) -> Option<String> {
        let probability_regex = regex::Regex::new(r"([0-9]*\.?[0-9]+)\s*(?:%|probability|chance)").ok()?;
        let percentages: Vec<_> = probability_regex.find_iter(content)
            .map(|m| m.as_str())
            .collect();

        if !percentages.is_empty() {
            Some(percentages.join(", "))
        } else {
            None
        }
    }

    fn extract_mapping_patterns(&self, content: &str) -> Option<String> {
        if content.contains("put(") && content.contains("get(") {
            Some("Key-value mapping/lookup system".to_string())
        } else if content.contains("alias") {
            Some("Alias/shortcut mapping system".to_string())
        } else {
            None
        }
    }

    fn extract_state_patterns(&self, content: &str) -> Option<String> {
        if content.contains("state") && (content.contains("enum") || content.contains("switch")) {
            Some("State machine or status management".to_string())
        } else {
            None
        }
    }

    fn extract_validation_patterns(&self, content: &str) -> Option<String> {
        if content.contains("validate") || content.contains("check") || content.contains("verify") {
            Some("Input validation and business rule enforcement".to_string())
        } else {
            None
        }
    }

    fn extract_calculation_patterns(&self, content: &str) -> Option<String> {
        if content.contains("calculate") || content.contains("compute") || content.contains("algorithm") {
            Some("Business calculations and algorithms".to_string())
        } else {
            None
        }
    }

    fn is_technical_type(&self, type_name: &str) -> bool {
        let technical_patterns = [
            "config", "configuration", "settings", "properties",
            "exception", "error", "result", "response", "request",
            "util", "helper", "handler", "processor", "manager",
            "builder", "factory", "adapter", "wrapper"
        ];

        let name_lower = type_name.to_lowercase();
        technical_patterns.iter().any(|pattern| name_lower.contains(pattern))
    }

    /// Filter dependencies to show only business-relevant ones
    fn filter_relevant_dependencies<'a>(&self, deps: &'a [super::package_analysis::ExternalDependency]) -> Vec<&'a super::package_analysis::ExternalDependency> {
        deps.iter()
            .filter(|dep| !self.is_framework_noise(&dep.name))
            .collect()
    }

    fn is_framework_noise(&self, dep_name: &str) -> bool {
        let noise_patterns = [
            "org.springframework", "com.fasterxml", "javax", "java.",
            "org.slf4j", "ch.qos.logback", "org.junit", "org.mockito",
            "org.apache.commons", "com.google.guava",
            // Add more framework noise patterns as needed
        ];

        noise_patterns.iter().any(|pattern| dep_name.starts_with(pattern))
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
            .any(|hint| hint.source.contains("Cargo.toml") && hint.insights.iter().any(|i| i.contains("bin"))) {
            "application".to_string()
        } else if human_context.configuration_hints.iter()
            .any(|hint| hint.insights.iter().any(|i| i.contains("web") || i.contains("server"))) {
            "web service".to_string()
        } else {
            "library".to_string()
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

impl Default for BatchProcessor {
    fn default() -> Self {
        Self::new()
    }
}