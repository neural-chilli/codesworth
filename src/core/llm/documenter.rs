use serde::{Serialize, Deserialize};
use std::collections::HashMap;

use crate::error::Result;
use super::super::{ParsedFile, ParsedModule};

/// Context information passed to LLM for documentation enhancement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentationContext {
    /// The parsed file being documented
    pub file: ParsedFile,

    /// The specific module being enhanced (if any)
    pub target_module: Option<ParsedModule>,

    /// Related files in the project for context
    pub related_files: Vec<ParsedFile>,

    /// Project-level information
    pub project_info: ProjectInfo,

    /// Architecture documentation for additional context
    pub architecture_docs: Option<ArchitectureDocs>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureDocs {
    /// High-level system overview
    pub system_overview: Option<String>,

    /// Key architectural decisions
    pub architectural_decisions: Vec<String>,

    /// Technology stack information
    pub technology_stack: Vec<String>,

    /// Design patterns used in the project
    pub design_patterns: Vec<String>,

    /// Integration points and external dependencies
    pub integrations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    /// Project name
    pub name: String,

    /// Project description (from README or config)
    pub description: Option<String>,

    /// Programming language
    pub language: String,

    /// Project type (library, application, service, etc.)
    pub project_type: Option<String>,
}

/// Request for LLM to enhance documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancementRequest {
    /// Type of enhancement requested
    pub enhancement_type: EnhancementType,

    /// Context information
    pub context: DocumentationContext,

    /// Current documentation content (if any)
    pub current_content: Option<String>,

    /// Specific areas to focus on
    pub focus_areas: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnhancementType {
    /// Generate overview documentation for a module/file
    ModuleOverview,

    /// Generate function/method documentation
    FunctionDocumentation,

    /// Generate usage examples
    UsageExamples,

    /// Explain architectural decisions
    ArchitecturalInsights,

    /// Generate implementation notes
    ImplementationDetails,

    /// Generate testing recommendations
    TestingStrategy,

    /// Custom enhancement with specific prompt
    Custom(String),
}

/// Response from LLM with enhanced documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancementResponse {
    /// The enhanced documentation content
    pub content: String,

    /// Confidence score (0.0 to 1.0) if available
    pub confidence: Option<f32>,

    /// Suggested improvements or additional context
    pub suggestions: Vec<String>,

    /// Metadata about the enhancement
    pub metadata: HashMap<String, String>,
}

/// Trait for LLM providers that can enhance documentation
#[async_trait::async_trait]
pub trait LlmDocumenter: Send + Sync {
    /// Enhance documentation using the LLM
    async fn enhance_documentation(&self, request: EnhancementRequest) -> Result<EnhancementResponse>;

    /// Get the provider name (e.g., "OpenAI GPT-4", "Anthropic Claude")
    fn provider_name(&self) -> &str;

    /// Get the model name being used
    fn model_name(&self) -> &str;

    /// Check if the provider is available (API key set, service reachable, etc.)
    async fn health_check(&self) -> Result<bool>;

    /// Get provider-specific capabilities
    fn capabilities(&self) -> DocumenterCapabilities;
}

#[derive(Debug, Clone)]
pub struct DocumenterCapabilities {
    /// Maximum context length in tokens
    pub max_context_tokens: Option<u32>,

    /// Maximum response length in tokens
    pub max_response_tokens: Option<u32>,

    /// Supported enhancement types
    pub supported_enhancements: Vec<EnhancementType>,

    /// Whether the provider supports streaming responses
    pub supports_streaming: bool,

    /// Whether the provider supports code understanding
    pub supports_code_analysis: bool,
}

impl Default for DocumenterCapabilities {
    fn default() -> Self {
        Self {
            max_context_tokens: Some(8000),
            max_response_tokens: Some(2000),
            supported_enhancements: vec![
                EnhancementType::ModuleOverview,
                EnhancementType::FunctionDocumentation,
                EnhancementType::UsageExamples,
                EnhancementType::ImplementationDetails,
            ],
            supports_streaming: false,
            supports_code_analysis: true,
        }
    }
}