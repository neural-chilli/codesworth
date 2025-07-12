// src/core/hierarchical_analyzer.rs
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
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

    /// Specific subdomain (e.g., "Interactive Character Simulation", "Order Management")
    pub subdomain: String,

    /// Primary user types
    pub user_types: Vec<String>,

    /// Core business concepts
    pub key_concepts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserWorkflow {
    /// Workflow name (e.g., "Character Interaction Flow")
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
pub struct ArchitecturalDecision {
    pub decision: String,
    pub rationale: String,
    pub alternatives_considered: Vec<String>,
    pub trade_offs: Vec<String>,
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

        // Phase 1: Extract comprehensive semantic intelligence
        let semantic_intelligence = self.extract_semantic_intelligence(packages).await?;

        // Phase 2: Build system understanding through progressive analysis
        let system_understanding = self.build_system_understanding(
            &semantic_intelligence,
            packages,
            human_context,
            system_context,
            llm_documenter,
        ).await?;

        Ok(system_understanding)
    }

    /// Extract comprehensive semantic intelligence from all code
    async fn extract_semantic_intelligence(&self, packages: &[PackageAnalysis]) -> Result<SemanticCodeIntelligence> {
        let mut intelligence = SemanticCodeIntelligence::new();

        for package in packages {
            for file in &package.files {
                // Extract entity definitions and relationships
                intelligence.entity_patterns.extend(self.extract_entity_patterns(&file.source_content));

                // Extract business logic patterns
                intelligence.business_logic.extend(self.extract_business_logic_patterns(&file.source_content));

                // Extract data mapping patterns
                intelligence.data_mappings.extend(self.extract_data_mappings(&file.source_content));

                // Extract workflow indicators
                intelligence.workflow_patterns.extend(self.extract_workflow_patterns(&file.source_content));

                // Extract domain vocabulary with context
                intelligence.domain_terms.extend(self.extract_contextual_vocabulary(&file.source_content));

                // Extract configuration patterns
                intelligence.configuration_patterns.extend(self.extract_configuration_patterns(&file.source_content));
            }
        }

        // Analyze cross-package patterns
        intelligence.system_patterns = self.analyze_system_level_patterns(packages);

        Ok(intelligence)
    }

    /// Build system understanding through progressive LLM analysis
    async fn build_system_understanding(
        &self,
        semantic_intelligence: &SemanticCodeIntelligence,
        packages: &[PackageAnalysis],
        human_context: &HumanContext,
        system_context: &SystemContext,
        llm_documenter: &dyn LlmDocumenter,
    ) -> Result<SystemUnderstanding> {

        // Step 1: Analyze business domain from semantic patterns
        let domain_analysis = self.analyze_business_domain_semantically(semantic_intelligence, llm_documenter).await?;

        // Step 2: Understand user workflows
        let workflow_analysis = self.analyze_user_workflows_semantically(semantic_intelligence, packages, llm_documenter).await?;

        // Step 3: Architecture overview
        let architecture_analysis = self.analyze_architecture_overview(packages, human_context, llm_documenter).await?;

        // Step 4: Build domain model
        let domain_model = self.build_domain_model(semantic_intelligence, &domain_analysis, llm_documenter).await?;

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

    /// Analyze business domain from semantic patterns
    async fn analyze_business_domain_semantically(
        &self,
        intelligence: &SemanticCodeIntelligence,
        llm_documenter: &dyn LlmDocumenter,
    ) -> Result<BusinessDomain> {

        // Build prompt with semantic analysis
        let prompt = format!(
            "Analyze this codebase to determine the ACTUAL business domain and purpose.\n\
            Focus on understanding what business problem this solves, not just technical implementation.\n\n\
            ENTITY PATTERNS:\n{}\n\n\
            BUSINESS LOGIC PATTERNS:\n{}\n\n\
            DATA MAPPING PATTERNS:\n{}\n\n\
            WORKFLOW PATTERNS:\n{}\n\n\
            DOMAIN VOCABULARY:\n{}\n\n\
            CONFIGURATION PATTERNS:\n{}\n\n\
            SYSTEM-LEVEL PATTERNS:\n{}\n\n\
            Based on this semantic analysis, determine:\n\
            1. What industry/domain does this system serve?\n\
            2. What specific business problem does it solve?\n\
            3. Who are the primary users and what do they accomplish?\n\
            4. What are the core business concepts and entities?\n\n\
            CRITICAL: Look for the ACTUAL business purpose, not generic technical functions.\n\
            Examples:\n\
            - If you see character/persona patterns + alias mappings + probability responses → Entertainment/Simulation\n\
            - If you see order/payment/cart patterns → E-commerce\n\
            - If you see account/transaction/balance patterns → Financial Services\n\
            - If you see patient/diagnosis/treatment patterns → Healthcare\n\n\
            Respond in JSON format:\n\
            {{\n\
              \"domain_type\": \"Specific industry/domain based on evidence\",\n\
              \"subdomain\": \"Specific business area based on patterns\",\n\
              \"user_types\": [\"Primary user type\", \"Secondary user type\"],\n\
              \"key_concepts\": [\"Business concept 1\", \"Business concept 2\"]\n\
            }}",
            intelligence.entity_patterns.join("\n"),
            intelligence.business_logic.join("\n"),
            intelligence.data_mappings.join("\n"),
            intelligence.workflow_patterns.join("\n"),
            self.format_domain_terms(&intelligence.domain_terms),
            intelligence.configuration_patterns.join("\n"),
            intelligence.system_patterns.join("\n")
        );

        // Make chunked LLM call if needed
        let response = self.make_chunked_llm_call(prompt, llm_documenter).await?;

        // Parse JSON response
        let domain: BusinessDomain = serde_json::from_str(&response.content)
            .unwrap_or_else(|_| BusinessDomain {
                domain_type: "Software Application".to_string(),
                subdomain: "Specialized Application".to_string(),
                user_types: vec!["End Users".to_string()],
                key_concepts: vec!["User Interaction".to_string(), "Data Processing".to_string()],
            });

        Ok(domain)
    }

    /// Analyze user workflows from semantic patterns
    async fn analyze_user_workflows_semantically(
        &self,
        intelligence: &SemanticCodeIntelligence,
        packages: &[PackageAnalysis],
        llm_documenter: &dyn LlmDocumenter,
    ) -> Result<Vec<UserWorkflow>> {

        // Build workflow analysis prompt
        let prompt = format!(
            "Analyze the user workflows in this system based on semantic patterns.\n\n\
            WORKFLOW PATTERNS:\n{}\n\n\
            BUSINESS LOGIC:\n{}\n\n\
            ENTITY RELATIONSHIPS:\n{}\n\n\
            DATA FLOWS:\n{}\n\n\
            Identify the main user workflows - what do users actually accomplish with this system?\n\
            Focus on business value, not just technical operations.\n\n\
            For each workflow, explain:\n\
            1. What business goal the user achieves\n\
            2. The key steps in the process\n\
            3. What business rules apply\n\
            4. Which packages handle each step\n\n\
            Respond in JSON format:\n\
            {{\n\
              \"workflows\": [\n\
                {{\n\
                  \"name\": \"Specific business workflow name\",\n\
                  \"steps\": [\n\
                    {{\n\
                      \"description\": \"What the user accomplishes\",\n\
                      \"technical_implementation\": \"How the code supports this\",\n\
                      \"business_rules\": [\"Rule 1\", \"Rule 2\"]\n\
                    }}\n\
                  ],\n\
                  \"involved_packages\": [\"package1\", \"package2\"]\n\
                }}\n\
              ]\n\
            }}",
            intelligence.workflow_patterns.join("\n"),
            intelligence.business_logic.join("\n"),
            intelligence.entity_patterns.join("\n"),
            self.extract_data_flows(packages)
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

    // Helper methods for semantic analysis

    fn extract_entity_patterns(&self, content: &str) -> Vec<String> {
        let mut patterns = Vec::new();

        // Look for class/struct definitions with context
        let lines: Vec<&str> = content.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if line.contains("class ") || line.contains("struct ") || line.contains("enum ") {
                if let Some(entity_name) = self.extract_entity_name(line) {
                    let context = self.extract_entity_context(&lines, i);
                    patterns.push(format!("{}: {}", entity_name, context));
                }
            }
        }

        patterns
    }

    fn extract_business_logic_patterns(&self, content: &str) -> Vec<String> {
        let mut patterns = Vec::new();

        // Look for probability/percentage patterns
        if let Ok(re) = regex::Regex::new(r"(probability|chance)\s*=\s*([0-9.]+)") {
            for cap in re.captures_iter(content) {
                let value = cap.get(2).map(|m| m.as_str()).unwrap_or("unknown");
                patterns.push(format!("Probability-based decision making: {}", value));
            }
        }

        // Look for random selection patterns
        if content.contains("random") && content.contains("size()") {
            patterns.push("Random selection from available options".to_string());
        }

        // Look for validation patterns
        if content.contains("validate") || content.contains("check") {
            patterns.push("Input validation and business rule enforcement".to_string());
        }

        // Look for processing patterns
        if content.contains("process") && (content.contains("Message") || content.contains("Request")) {
            patterns.push("Request/message processing workflow".to_string());
        }

        patterns
    }

    fn extract_data_mappings(&self, content: &str) -> Vec<String> {
        let mut mappings = Vec::new();

        // Look for put/get mapping patterns
        if content.contains("put(") && content.contains("get(") {
            // Extract specific mapping examples
            let lines: Vec<&str> = content.lines().collect();
            for line in lines {
                if line.contains("put(") && line.contains("\"") {
                    if let Some(mapping) = self.extract_mapping_example(line) {
                        mappings.push(mapping);
                    }
                }
            }
        }

        // Look for alias patterns
        if content.to_lowercase().contains("alias") {
            mappings.push("Alias/shortcut mapping system".to_string());
        }

        mappings
    }

    fn extract_workflow_patterns(&self, content: &str) -> Vec<String> {
        let mut patterns = Vec::new();

        // WebSocket communication patterns
        if content.contains("WebSocket") || content.contains("@MessageMapping") {
            patterns.push("Real-time bidirectional communication".to_string());
        }

        // HTTP endpoint patterns
        if content.contains("@GetMapping") || content.contains("@PostMapping") {
            patterns.push("HTTP request/response handling".to_string());
        }

        // Event handling patterns
        if content.contains("handle") && content.contains("Message") {
            patterns.push("Event-driven message handling".to_string());
        }

        // Trigger patterns
        if content.contains("trigger") || content.contains("followup") {
            patterns.push("Automatic follow-up/triggering logic".to_string());
        }

        patterns
    }

    fn extract_contextual_vocabulary(&self, content: &str) -> Vec<(String, String)> {
        let mut vocabulary = Vec::new();

        // Extract domain-specific terms with their usage context
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

        // Extract terms that appear multiple times
        for (word, count) in word_counts {
            if count >= 2 {
                let context = self.extract_word_usage_context(&word, content);
                vocabulary.push((word, context));
            }
        }

        vocabulary.sort_by(|a, b| b.1.len().cmp(&a.1.len())); // Sort by context richness
        vocabulary.truncate(8); // Keep most relevant terms
        vocabulary
    }

    fn extract_configuration_patterns(&self, content: &str) -> Vec<String> {
        let mut patterns = Vec::new();

        // WebSocket configuration
        if content.contains("WebSocketConfig") || content.contains("STOMP") {
            patterns.push("WebSocket real-time communication setup".to_string());
        }

        // Spring configuration
        if content.contains("@Configuration") || content.contains("@Bean") {
            patterns.push("Spring framework dependency injection".to_string());
        }

        // Message broker configuration
        if content.contains("MessageBroker") || content.contains("enableSimpleBroker") {
            patterns.push("Message broker for real-time messaging".to_string());
        }

        patterns
    }

    fn analyze_system_level_patterns(&self, packages: &[PackageAnalysis]) -> Vec<String> {
        let mut patterns = Vec::new();

        // Analyze package relationships for system patterns
        let package_names: Vec<_> = packages.iter().map(|p| &p.package_name).collect();

        // Look for layered architecture patterns
        if package_names.iter().any(|name| name.contains("controller")) &&
            package_names.iter().any(|name| name.contains("service")) &&
            package_names.iter().any(|name| name.contains("domain")) {
            patterns.push("Layered architecture with separation of concerns".to_string());
        }

        // Look for configuration patterns
        if package_names.iter().any(|name| name.contains("config")) {
            patterns.push("Centralized configuration management".to_string());
        }

        // Look for domain-driven design patterns
        if package_names.iter().any(|name| name.contains("domain")) {
            patterns.push("Domain-driven design with business entity modeling".to_string());
        }

        // Analyze dependencies for system-level insights
        let all_dependencies: Vec<_> = packages.iter()
            .flat_map(|p| &p.dependencies.external_deps)
            .collect();

        if all_dependencies.iter().any(|dep| dep.name.to_lowercase().contains("websocket")) {
            patterns.push("Real-time communication capabilities throughout system".to_string());
        }

        patterns
    }

    // Helper methods for semantic analysis

    fn extract_entity_name(&self, line: &str) -> Option<String> {
        // Extract class/struct name
        if let Ok(re) = regex::Regex::new(r"(?:class|struct)\s+(\w+)") {
            if let Some(cap) = re.captures(line) {
                return cap.get(1).map(|m| m.as_str().to_string());
            }
        }
        None
    }

    fn extract_entity_context(&self, lines: &[&str], index: usize) -> String {
        // Look at surrounding lines for context
        let start = index.saturating_sub(2);
        let end = (index + 3).min(lines.len());

        for i in start..end {
            if lines[i].trim().starts_with("//") || lines[i].trim().starts_with("*") {
                let comment = lines[i].trim()
                    .trim_start_matches("//")
                    .trim_start_matches("*")
                    .trim();
                if comment.len() > 10 {
                    return comment.to_string();
                }
            }
        }

        "business entity".to_string()
    }

    fn extract_mapping_example(&self, line: &str) -> Option<String> {
        // Extract put("key", "value") patterns
        if let Ok(re) = regex::Regex::new(r#"put\("([^"]+)",\s*"([^"]+)"\)"#) {
            if let Some(cap) = re.captures(line) {
                let key = cap.get(1)?.as_str();
                let value = cap.get(2)?.as_str();
                return Some(format!("{}→{}", key, value));
            }
        }
        None
    }

    fn extract_word_usage_context(&self, word: &str, content: &str) -> String {
        // Find lines containing the word
        for line in content.lines() {
            if line.to_lowercase().contains(word) {
                // Let's fix the temporary value issue by using a let binding
                let line_trim = line.trim();
                let cleaned = line_trim
                    .replace("public", "")
                    .replace("private", "")
                    .replace("class", "")
                    .replace("//", "");
                let final_cleaned = cleaned.trim();
                
                if final_cleaned.len() > word.len() + 10 {
                    return final_cleaned.to_string();
                }
            }
        }
        "domain concept".to_string()
    }

    fn format_domain_terms(&self, terms: &[(String, String)]) -> String {
        terms.iter()
            .map(|(term, context)| format!("{}: {}", term, context))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn extract_data_flows(&self, packages: &[PackageAnalysis]) -> String {
        let mut flows = Vec::new();

        for package in packages {
            for entry_point in &package.public_api.entry_points {
                flows.push(format!("{}: {:?}", entry_point.name, entry_point.entry_type));
            }
        }

        flows.join("\n")
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
                primary_language: "Rust".to_string(),
                frameworks: vec!["Web Framework".to_string()],
                key_libraries: vec![],
                data_storage: vec![],
                communication_protocols: vec!["HTTP".to_string()],
            },
            non_functional_attributes: vec![],
        })
    }

    async fn build_domain_model(
        &self,
        _intelligence: &SemanticCodeIntelligence,
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

/// Semantic intelligence extracted from code analysis
#[derive(Debug, Clone)]
struct SemanticCodeIntelligence {
    /// Entity relationships found in the code
    pub entity_patterns: Vec<String>,

    /// Business logic patterns identified
    pub business_logic: Vec<String>,

    /// Data flow patterns
    pub data_mappings: Vec<String>,

    /// Workflow patterns
    pub workflow_patterns: Vec<String>,

    /// Domain-specific vocabulary with context
    pub domain_terms: Vec<(String, String)>,

    /// Configuration insights
    pub configuration_patterns: Vec<String>,

    /// System-level patterns
    pub system_patterns: Vec<String>,
}

impl SemanticCodeIntelligence {
    fn new() -> Self {
        Self {
            entity_patterns: Vec::new(),
            business_logic: Vec::new(),
            data_mappings: Vec::new(),
            workflow_patterns: Vec::new(),
            domain_terms: Vec::new(),
            configuration_patterns: Vec::new(),
            system_patterns: Vec::new(),
        }
    }
}