// src/core/hierarchical_analyzer.rs
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::core::system_overview_generator::ArchitecturalDecision;
use crate::error::Result;
use super::{
    LlmDocumenter, PackageAnalysis, HumanContext, SystemContext,
    package_analysis::*
};

/// Multi-level analysis system that builds understanding from code up to full system
pub struct HierarchicalAnalyzer {
    context_window_limit: usize,
    overlap_buffer: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemUnderstanding {
    /// High-level system purpose and domain
    pub system_purpose: String,

    /// Business domain and user types
    pub business_domain: BusinessDomain,

    /// Core workflows and user journeys
    pub user_workflows: Vec<UserWorkflow>,

    /// System architecture themes
    pub architecture_overview: ArchitectureOverview,

    /// Key business entities and their relationships
    pub domain_model: DomainModel,

    /// Critical insights and gotchas
    pub system_insights: Vec<SystemInsight>,

    /// Confidence in the analysis
    pub confidence_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessDomain {
    /// What industry/domain (e.g., "Entertainment/Simulation", "E-commerce", "FinTech")
    pub domain_type: String,

    /// Specific subdomain (e.g., "Star Trek Role-Playing Chat", "Order Management")
    pub subdomain: String,

    /// Primary user types
    pub user_types: Vec<String>,

    /// Core business concepts
    pub key_concepts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserWorkflow {
    /// Workflow name (e.g., "Character Chat Simulation")
    pub name: String,

    /// Step-by-step user journey
    pub steps: Vec<WorkflowStep>,

    /// Packages involved in this workflow
    pub involved_packages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub description: String,
    pub technical_implementation: String,
    pub business_rules: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureOverview {
    /// Architectural pattern (MVC, Microservices, Event-Driven, etc.)
    pub pattern: String,

    /// Key architectural decisions and rationale
    pub key_decisions: Vec<ArchitecturalDecision>,

    /// Technology stack summary
    pub technology_summary: TechnologySummary,

    /// Scalability and performance characteristics
    pub non_functional_attributes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechnologySummary {
    pub primary_language: String,
    pub frameworks: Vec<String>,
    pub key_libraries: Vec<String>,
    pub data_storage: Vec<String>,
    pub communication_protocols: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainModel {
    /// Core business entities
    pub entities: Vec<DomainEntity>,

    /// Relationships between entities
    pub relationships: Vec<EntityRelationship>,

    /// Business rules and constraints
    pub business_rules: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainEntity {
    pub name: String,
    pub description: String,
    pub attributes: Vec<String>,
    pub business_significance: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityRelationship {
    pub from_entity: String,
    pub to_entity: String,
    pub relationship_type: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInsight {
    pub category: InsightCategory,
    pub insight: String,
    pub impact: InsightImpact,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InsightCategory {
    BusinessPurpose,
    ArchitecturalDecision,
    PerformanceCharacteristic,
    SecurityConsideration,
    MaintenanceComplexity,
    IntegrationPattern,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InsightImpact {
    Critical,    // Essential for understanding the system
    Important,   // Significantly affects development/maintenance
    Useful,      // Good to know for optimization
}

impl HierarchicalAnalyzer {
    pub fn new(context_window_limit: usize) -> Self {
        Self {
            context_window_limit,
            overlap_buffer: context_window_limit / 10, // 10% overlap between chunks
        }
    }

    /// Make LLM call with automatic chunking for large prompts (public method)
    pub async fn make_chunked_llm_call(
        &self,
        prompt: String,
        llm_documenter: &dyn LlmDocumenter,
    ) -> Result<super::EnhancementResponse> {

        // Estimate token count (rough: 4 chars ≈ 1 token)
        let estimated_tokens = prompt.len() / 4;

        if estimated_tokens <= self.context_window_limit {
            // Fits in single call
            let request = super::EnhancementRequest {
                enhancement_type: super::EnhancementType::Custom(prompt),
                context: self.create_minimal_context()?,
                current_content: None,
                focus_areas: vec!["business_domain".to_string()],
            };

            return llm_documenter.enhance_documentation(request).await;
        }

        // Need to chunk the prompt
        self.process_chunked_analysis(prompt, llm_documenter).await
    }

    /// Build comprehensive system understanding through multi-pass analysis
    pub async fn analyze_full_system(
        &self,
        packages: &[PackageAnalysis],
        human_context: &HumanContext,
        system_context: &SystemContext,
        llm_documenter: &dyn LlmDocumenter,
    ) -> Result<SystemUnderstanding> {

        // Phase 1: Extract all string literals and critical code patterns
        let code_intelligence = self.extract_comprehensive_code_intelligence(packages).await?;

        // Phase 2: Build system understanding through iterative analysis
        let system_understanding = self.build_system_understanding(
            &code_intelligence,
            packages,
            human_context,
            system_context,
            llm_documenter,
        ).await?;

        Ok(system_understanding)
    }

    /// Extract comprehensive intelligence from all code
    async fn extract_comprehensive_code_intelligence(&self, packages: &[PackageAnalysis]) -> Result<CodeIntelligence> {
        let mut intelligence = CodeIntelligence::new();

        for package in packages {
            for file in &package.files {
                // Extract ALL string literals (this is what we were missing!)
                intelligence.string_literals.extend(self.extract_string_literals(&file.source_content));

                // Extract function calls and their contexts
                intelligence.function_calls.extend(self.extract_function_calls(&file.source_content));

                // Extract class/struct definitions with their full context
                intelligence.type_definitions.extend(self.extract_type_definitions(&file.source_content));

                // Extract comments and documentation
                intelligence.comments.extend(self.extract_all_comments(&file.source_content));

                // Extract variable names and their contexts
                intelligence.variable_patterns.extend(self.extract_variable_patterns(&file.source_content));
            }
        }

        // Analyze patterns across all extracted intelligence
        intelligence.business_patterns = self.analyze_cross_cutting_patterns(&intelligence);

        Ok(intelligence)
    }

    /// Build system understanding through progressive LLM analysis
    async fn build_system_understanding(
        &self,
        code_intelligence: &CodeIntelligence,
        packages: &[PackageAnalysis],
        human_context: &HumanContext,
        system_context: &SystemContext,
        llm_documenter: &dyn LlmDocumenter,
    ) -> Result<SystemUnderstanding> {

        // Step 1: Analyze business domain from code intelligence
        let domain_analysis = self.analyze_business_domain(code_intelligence, llm_documenter).await?;

        // Step 2: Understand user workflows
        let workflow_analysis = self.analyze_user_workflows(code_intelligence, packages, llm_documenter).await?;

        // Step 3: Architecture overview
        let architecture_analysis = self.analyze_architecture_overview(packages, human_context, llm_documenter).await?;

        // Step 4: Build domain model
        let domain_model = self.build_domain_model(code_intelligence, &domain_analysis, llm_documenter).await?;

        // Step 5: Generate system insights
        let system_insights = self.generate_system_insights(
            &domain_analysis,
            &workflow_analysis,
            &architecture_analysis,
            llm_documenter,
        ).await?;

        // Step 6: Synthesize final understanding
        let system_purpose = self.synthesize_system_purpose(
            &domain_analysis,
            &workflow_analysis,
            human_context,
            llm_documenter,
        ).await?;

        Ok(SystemUnderstanding {
            system_purpose,
            business_domain: domain_analysis,
            user_workflows: workflow_analysis,
            architecture_overview: architecture_analysis,
            domain_model,
            system_insights,
            confidence_score: 0.85, // TODO: Calculate based on evidence quality
        })
    }

    /// Analyze business domain from comprehensive code intelligence
    async fn analyze_business_domain(
        &self,
        intelligence: &CodeIntelligence,
        llm_documenter: &dyn LlmDocumenter,
    ) -> Result<BusinessDomain> {

        // Build prompt with ALL the extracted intelligence
        let prompt = format!(
            "Analyze this codebase to determine the business domain and purpose.\n\n\
            STRING LITERALS FROM CODE:\n{}\n\n\
            FUNCTION CALLS AND CONTEXTS:\n{}\n\n\
            TYPE DEFINITIONS:\n{}\n\n\
            COMMENTS AND DOCUMENTATION:\n{}\n\n\
            VARIABLE PATTERNS:\n{}\n\n\
            CROSS-CUTTING PATTERNS:\n{}\n\n\
            Based on this comprehensive code analysis, determine:\n\
            1. What industry/domain is this system for?\n\
            2. What specific business problem does it solve?\n\
            3. Who are the primary users?\n\
            4. What are the core business concepts?\n\n\
            Respond in JSON format:\n\
            {{\n\
              \"domain_type\": \"Primary industry/domain\",\n\
              \"subdomain\": \"Specific business area\",\n\
              \"user_types\": [\"Primary user type\", \"Secondary user type\"],\n\
              \"key_concepts\": [\"Core concept 1\", \"Core concept 2\"]\n\
            }}",
            intelligence.string_literals.join("\n"),
            intelligence.function_calls.join("\n"),
            intelligence.type_definitions.join("\n"),
            intelligence.comments.join("\n"),
            intelligence.variable_patterns.join("\n"),
            intelligence.business_patterns.join("\n")
        );

        // Make chunked LLM call if needed
        let response = self.make_chunked_llm_call(prompt, llm_documenter).await?;

        // Parse JSON response
        let domain: BusinessDomain = serde_json::from_str(&response.content)
            .unwrap_or_else(|_| BusinessDomain {
                domain_type: "Unknown".to_string(),
                subdomain: "Unspecified".to_string(),
                user_types: vec!["Users".to_string()],
                key_concepts: vec!["Data".to_string(), "Processing".to_string()],
            });

        Ok(domain)
    }

    /// Analyze user workflows from code patterns
    async fn analyze_user_workflows(
        &self,
        intelligence: &CodeIntelligence,
        packages: &[PackageAnalysis],
        llm_documenter: &dyn LlmDocumenter,
    ) -> Result<Vec<UserWorkflow>> {

        // Build workflow analysis prompt
        let prompt = format!(
            "Analyze the user workflows in this system based on the code patterns.\n\n\
            ENTRY POINTS:\n{}\n\n\
            FUNCTION CALL CHAINS:\n{}\n\n\
            DATA FLOW PATTERNS:\n{}\n\n\
            Identify the main user workflows - what do users actually DO with this system?\n\
            For each workflow, trace the technical implementation through the packages.\n\n\
            Respond in JSON format:\n\
            {{\n\
              \"workflows\": [\n\
                {{\n\
                  \"name\": \"Primary workflow name\",\n\
                  \"steps\": [\n\
                    {{\n\
                      \"description\": \"What the user does\",\n\
                      \"technical_implementation\": \"How the code handles this\",\n\
                      \"business_rules\": [\"Rule 1\", \"Rule 2\"]\n\
                    }}\n\
                  ],\n\
                  \"involved_packages\": [\"package1\", \"package2\"]\n\
                }}\n\
              ]\n\
            }}",
            self.extract_entry_points(packages),
            intelligence.function_calls.join("\n"),
            self.extract_data_flow_patterns(intelligence)
        );

        let response = self.make_chunked_llm_call(prompt, llm_documenter).await?;

        // Parse workflows
        if let Ok(workflow_data) = serde_json::from_str::<serde_json::Value>(&response.content) {
            if let Some(workflows_array) = workflow_data.get("workflows").and_then(|w| w.as_array()) {
                let mut workflows = Vec::new();
                for workflow_json in workflows_array {
                    if let Ok(workflow) = serde_json::from_value::<UserWorkflow>(workflow_json.clone()) {
                        workflows.push(workflow);
                    }
                }
                return Ok(workflows);
            }
        }

        // Fallback
        Ok(vec![UserWorkflow {
            name: "Primary User Interaction".to_string(),
            steps: vec![],
            involved_packages: packages.iter().map(|p| p.package_name.clone()).collect(),
        }])
    }

    /// Make LLM call with automatic chunking for large prompts
    async fn make_chunked_llm_call_private(
        &self,
        prompt: String,
        llm_documenter: &dyn LlmDocumenter,
    ) -> Result<super::EnhancementResponse> {
        self.make_chunked_llm_call(prompt, llm_documenter).await
    }

    /// Process analysis in chunks with progressive summarization
    async fn process_chunked_analysis(
        &self,
        full_prompt: String,
        llm_documenter: &dyn LlmDocumenter,
    ) -> Result<super::EnhancementResponse> {

        let chunks = self.split_prompt_into_chunks(&full_prompt);
        let mut accumulated_insights = Vec::new();

        for (i, chunk) in chunks.iter().enumerate() {
            let chunk_prompt = if i == 0 {
                format!("{}\n\nThis is chunk 1 of {}. Provide initial analysis.", chunk, chunks.len())
            } else {
                format!(
                    "Previous insights:\n{}\n\n\
                    Additional information (chunk {} of {}):\n{}\n\n\
                    Update your analysis based on this additional information.",
                    accumulated_insights.join("\n"),
                    i + 1,
                    chunks.len(),
                    chunk
                )
            };

            let request = super::EnhancementRequest {
                enhancement_type: super::EnhancementType::Custom(chunk_prompt),
                context: self.create_minimal_context()?,
                current_content: None,
                focus_areas: vec!["progressive_analysis".to_string()],
            };

            let response = llm_documenter.enhance_documentation(request).await?;
            accumulated_insights.push(response.content.clone());

            // For final chunk, return the complete response
            if i == chunks.len() - 1 {
                return Ok(response);
            }
        }

        // Shouldn't reach here, but return last response as fallback
        Ok(super::EnhancementResponse {
            content: accumulated_insights.join("\n"),
            confidence: Some(0.7),
            suggestions: vec!["Analysis completed through chunking".to_string()],
            metadata: HashMap::new(),
        })
    }

    // Helper methods for code intelligence extraction

    fn extract_string_literals(&self, content: &str) -> Vec<String> {
        let mut literals = Vec::new();

        // Extract quoted strings
        if let Ok(re) = regex::Regex::new(r#""([^"\\]|\\.)*""#) {
            for mat in re.find_iter(content) {
                let literal = mat.as_str();
                // Filter out short or obvious technical strings
                if literal.len() > 8 && !self.is_technical_string(literal) {
                    literals.push(literal.to_string());
                }
            }
        }

        literals
    }

    fn extract_function_calls(&self, content: &str) -> Vec<String> {
        let mut calls = Vec::new();

        // Extract function call patterns
        if let Ok(re) = regex::Regex::new(r"(\w+)\s*\([^)]*\)") {
            for mat in re.find_iter(content) {
                let call = mat.as_str();
                if !self.is_technical_function_call(call) {
                    calls.push(call.to_string());
                }
            }
        }

        calls
    }

    fn extract_type_definitions(&self, content: &str) -> Vec<String> {
        let mut types = Vec::new();

        // Extract class/struct/enum definitions with some context
        let lines: Vec<&str> = content.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if line.contains("class ") || line.contains("struct ") || line.contains("enum ") {
                // Include some context around the definition
                let start = i.saturating_sub(2);
                let end = (i + 3).min(lines.len());
                let context = lines[start..end].join("\n");
                types.push(context);
            }
        }

        types
    }

    fn extract_all_comments(&self, content: &str) -> Vec<String> {
        let mut comments = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with("*") {
                let comment = trimmed.trim_start_matches("//")
                    .trim_start_matches("/*")
                    .trim_start_matches("*")
                    .trim();
                if comment.len() > 10 {
                    comments.push(comment.to_string());
                }
            }
        }

        comments
    }

    fn extract_variable_patterns(&self, content: &str) -> Vec<String> {
        let mut patterns = Vec::new();

        // Extract variable assignments that might reveal business logic
        if let Ok(re) = regex::Regex::new(r"(\w+)\s*=\s*([^;]+);") {
            for mat in re.find_iter(content) {
                let assignment = mat.as_str();
                if self.seems_business_relevant(assignment) {
                    patterns.push(assignment.to_string());
                }
            }
        }

        patterns
    }

    fn analyze_cross_cutting_patterns(&self, intelligence: &CodeIntelligence) -> Vec<String> {
        let mut patterns = Vec::new();

        // Analyze patterns across all extracted intelligence
        let all_text = format!(
            "{} {} {} {}",
            intelligence.string_literals.join(" "),
            intelligence.function_calls.join(" "),
            intelligence.comments.join(" "),
            intelligence.variable_patterns.join(" ")
        );

        // Look for business domain indicators
        if all_text.to_lowercase().contains("star trek") ||
            (all_text.contains("kirk") && all_text.contains("enterprise")) {
            patterns.push("Star Trek universe simulation detected".to_string());
        }

        // Look for chat/communication patterns
        if all_text.contains("chat") || all_text.contains("message") {
            patterns.push("Communication/messaging system detected".to_string());
        }

        // Look for AI/ML patterns
        if all_text.contains("ai ") || all_text.contains("prompt") || all_text.contains("model") {
            patterns.push("AI/ML integration detected".to_string());
        }

        patterns
    }

    // Utility methods

    fn is_technical_string(&self, s: &str) -> bool {
        let s_lower = s.to_lowercase();
        s_lower.contains("error") || s_lower.contains("exception") ||
            s_lower.contains("debug") || s_lower.contains("log") ||
            s.len() < 8
    }

    fn is_technical_function_call(&self, call: &str) -> bool {
        let call_lower = call.to_lowercase();
        call_lower.starts_with("get") || call_lower.starts_with("set") ||
            call_lower.contains("debug") || call_lower.contains("log")
    }

    fn seems_business_relevant(&self, assignment: &str) -> bool {
        let assignment_lower = assignment.to_lowercase();
        !assignment_lower.contains("debug") && !assignment_lower.contains("log") &&
            assignment.len() > 20
    }

    fn split_prompt_into_chunks(&self, prompt: &str) -> Vec<String> {
        let chunk_size = self.context_window_limit * 4; // Convert tokens to chars
        let mut chunks = Vec::new();

        if prompt.len() <= chunk_size {
            chunks.push(prompt.to_string());
            return chunks;
        }

        let mut start = 0;
        while start < prompt.len() {
            let end = (start + chunk_size - self.overlap_buffer).min(prompt.len());

            // Try to break at sentence boundaries
            let chunk_end = if end < prompt.len() {
                prompt[start..end].rfind('.').map(|pos| start + pos + 1).unwrap_or(end)
            } else {
                end
            };

            chunks.push(prompt[start..chunk_end].to_string());
            start = chunk_end.saturating_sub(self.overlap_buffer);
        }

        chunks
    }

    fn create_minimal_context(&self) -> Result<super::DocumentationContext> {
        // Create minimal context for LLM calls
        use std::path::PathBuf;

        Ok(super::DocumentationContext {
            file: super::ParsedFile {
                path: PathBuf::from("analysis"),
                language: "analysis".to_string(),
                content_hash: "analysis".to_string(),
                modified_time: std::time::SystemTime::now(),
                modules: vec![],
                file_docs: None,
                source_content: "".to_string(),
            },
            target_module: None,
            related_files: vec![],
            project_info: super::ProjectInfo {
                name: "System Analysis".to_string(),
                description: None,
                language: "multi".to_string(),
                project_type: Some("analysis".to_string()),
            },
            architecture_docs: None,
        })
    }

    // Placeholder implementations for missing methods
    fn extract_entry_points(&self, packages: &[PackageAnalysis]) -> String {
        let mut entry_points = Vec::new();
        for package in packages {
            for entry_point in &package.public_api.entry_points {
                entry_points.push(format!("{}: {:?}", entry_point.name, entry_point.entry_type));
            }
        }
        entry_points.join("\n")
    }

    fn extract_data_flow_patterns(&self, intelligence: &CodeIntelligence) -> String {
        // Simple implementation - could be enhanced
        intelligence.function_calls.join(" -> ")
    }

    // Placeholder implementations for other analysis methods
    async fn analyze_architecture_overview(
        &self,
        _packages: &[PackageAnalysis],
        _human_context: &HumanContext,
        _llm_documenter: &dyn LlmDocumenter,
    ) -> Result<ArchitectureOverview> {
        Ok(ArchitectureOverview {
            pattern: "Layered Architecture".to_string(),
            key_decisions: vec![],
            technology_summary: TechnologySummary {
                primary_language: "Java".to_string(),
                frameworks: vec!["Spring Boot".to_string()],
                key_libraries: vec![],
                data_storage: vec![],
                communication_protocols: vec!["WebSocket".to_string()],
            },
            non_functional_attributes: vec![],
        })
    }

    async fn build_domain_model(
        &self,
        _intelligence: &CodeIntelligence,
        _domain_analysis: &BusinessDomain,
        _llm_documenter: &dyn LlmDocumenter,
    ) -> Result<DomainModel> {
        Ok(DomainModel {
            entities: vec![],
            relationships: vec![],
            business_rules: vec![],
        })
    }

    async fn generate_system_insights(
        &self,
        _domain_analysis: &BusinessDomain,
        _workflow_analysis: &[UserWorkflow],
        _architecture_analysis: &ArchitectureOverview,
        _llm_documenter: &dyn LlmDocumenter,
    ) -> Result<Vec<SystemInsight>> {
        Ok(vec![])
    }

    async fn synthesize_system_purpose(
        &self,
        domain_analysis: &BusinessDomain,
        _workflow_analysis: &[UserWorkflow],
        _human_context: &HumanContext,
        _llm_documenter: &dyn LlmDocumenter,
    ) -> Result<String> {
        Ok(format!(
            "{} system in the {} domain",
            domain_analysis.subdomain,
            domain_analysis.domain_type
        ))
    }
}

/// Comprehensive code intelligence extracted from all source files
#[derive(Debug, Clone)]
struct CodeIntelligence {
    /// All string literals found in the code
    pub string_literals: Vec<String>,

    /// Function calls with context
    pub function_calls: Vec<String>,

    /// Type definitions with context
    pub type_definitions: Vec<String>,

    /// Comments and documentation
    pub comments: Vec<String>,

    /// Variable assignment patterns
    pub variable_patterns: Vec<String>,

    /// Cross-cutting business patterns identified
    pub business_patterns: Vec<String>,
}

impl CodeIntelligence {
    fn new() -> Self {
        Self {
            string_literals: Vec::new(),
            function_calls: Vec::new(),
            type_definitions: Vec::new(),
            comments: Vec::new(),
            variable_patterns: Vec::new(),
            business_patterns: Vec::new(),
        }
    }
}