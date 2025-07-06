use std::collections::HashMap;
use async_trait::async_trait;
use serde_json::json;

use crate::error::{CodesworthError, Result};
use crate::config::LlmConfig;
use super::documenter::{
    LlmDocumenter, EnhancementRequest, EnhancementResponse, DocumenterCapabilities,
    EnhancementType
};

/// Factory function to create the appropriate LLM documenter based on config
pub fn create_documenter(config: &LlmConfig) -> Result<Box<dyn LlmDocumenter>> {
    if !config.enabled {
        return Err(CodesworthError::Config("LLM integration is disabled".to_string()));
    }

    match config.provider.as_str() {
        "rig-openai" | "rig-anthropic" | "rig-google" | "rig-ollama" => {
            Ok(Box::new(RigProvider::new(config)?))
        }
        "cortex-gemini" | "cortex-claude" => {
            Ok(Box::new(CortexProvider::new(config)?))
        }
        _ => Err(CodesworthError::Config(
            format!("Unsupported LLM provider: {}", config.provider)
        )),
    }
}

/// Rig-based LLM provider (OpenAI, Anthropic, Google, Ollama)
pub struct RigProvider {
    config: LlmConfig,
    // TODO: Add rig client when implementing specific providers
}

impl RigProvider {
    pub fn new(config: &LlmConfig) -> Result<Self> {
        // Validate configuration
        if config.api_key.is_none() && !config.provider.contains("ollama") {
            return Err(CodesworthError::Config(
                "API key required for external LLM providers".to_string()
            ));
        }

        Ok(Self {
            config: config.clone(),
        })
    }

    async fn call_openai_api(&self, prompt: &str) -> Result<EnhancementResponse> {
        let api_key = self.config.api_key.as_ref()
            .ok_or_else(|| CodesworthError::Config("OpenAI API key not set".to_string()))?;

        let client = reqwest::Client::new();

        let payload = json!({
            "model": self.config.model,
            "messages": [
                {
                    "role": "system",
                    "content": "You are an expert software documentation assistant. Generate clear, professional documentation that helps developers understand and use the code effectively."
                },
                {
                    "role": "user", 
                    "content": prompt
                }
            ],
            "max_tokens": self.config.max_tokens.unwrap_or(2000),
            "temperature": self.config.temperature.unwrap_or(0.3)
        });

        let response = client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| CodesworthError::Parser(format!("OpenAI API request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(CodesworthError::Parser(
                format!("OpenAI API error {}: {}", status, error_text)
            ));
        }

        let response_data: serde_json::Value = response.json().await
            .map_err(|e| CodesworthError::Parser(format!("Failed to parse OpenAI response: {}", e)))?;

        let content = response_data["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("Failed to extract content from OpenAI response")
            .to_string();

        Ok(EnhancementResponse {
            content,
            confidence: Some(0.9),
            suggestions: vec![],
            metadata: {
                let mut map = HashMap::new();
                map.insert("provider".to_string(), "OpenAI".to_string());
                map.insert("model".to_string(), self.config.model.clone());
                if let Some(usage) = response_data.get("usage") {
                    map.insert("tokens_used".to_string(), usage["total_tokens"].to_string());
                }
                map
            },
        })
    }

    fn build_enhancement_prompt(&self, request: &EnhancementRequest) -> String {
        let context = &request.context;
        let file = &context.file;

        let mut prompt = String::new();

        // System context
        prompt.push_str(&format!(
            "You are an expert software documentation assistant. Your task is to enhance documentation for a {} project.\n\n",
            context.project_info.language
        ));

        // Project context
        prompt.push_str(&format!(
            "Project: {}\n",
            context.project_info.name
        ));

        if let Some(desc) = &context.project_info.description {
            prompt.push_str(&format!("Project Description: {}\n", desc));
        }

        // Architecture context
        if let Some(arch_docs) = &context.architecture_docs {
            prompt.push_str("\n=== ARCHITECTURAL CONTEXT ===\n");

            if let Some(overview) = &arch_docs.system_overview {
                prompt.push_str(&format!("System Overview: {}\n", overview));
            }

            if !arch_docs.technology_stack.is_empty() {
                prompt.push_str(&format!("Technology Stack: {}\n", arch_docs.technology_stack.join(", ")));
            }

            if !arch_docs.design_patterns.is_empty() {
                prompt.push_str(&format!("Design Patterns: {}\n", arch_docs.design_patterns.join(", ")));
            }

            if !arch_docs.integrations.is_empty() {
                prompt.push_str(&format!("Key Integrations: {}\n", arch_docs.integrations.join(", ")));
            }

            if !arch_docs.architectural_decisions.is_empty() {
                prompt.push_str("Architectural Decisions:\n");
                for decision in &arch_docs.architectural_decisions {
                    prompt.push_str(&format!("- {}\n", decision));
                }
            }
            prompt.push_str("=== END ARCHITECTURAL CONTEXT ===\n");
        }

        prompt.push_str("\n");

        // File context
        prompt.push_str(&format!(
            "File: {}\n",
            file.path.display()
        ));

        if let Some(docs) = &file.file_docs {
            prompt.push_str(&format!("Existing file documentation:\n{}\n\n", docs));
        }

        // Code structure
        prompt.push_str("Code structure:\n");
        for module in &file.modules {
            prompt.push_str(&format!(
                "- {} ({}): {}\n",
                module.name,
                module.item_type,
                module.docs.as_deref().unwrap_or("No documentation")
            ));

            for child in &module.children {
                prompt.push_str(&format!(
                    "  - {} ({}): {}\n",
                    child.name,
                    child.item_type,
                    child.docs.as_deref().unwrap_or("No documentation")
                ));
            }
        }

        prompt.push_str("\n");

        // Enhancement-specific instructions
        match &request.enhancement_type {
            EnhancementType::ModuleOverview => {
                prompt.push_str("Generate a clear, concise overview of what this module does, why it exists, and how it fits into the larger system. Focus on:\n");
                prompt.push_str("- Purpose and responsibilities\n");
                prompt.push_str("- Key architectural decisions\n");
                prompt.push_str("- How it relates to other components\n");
                prompt.push_str("- Design patterns used\n");
            }
            EnhancementType::FunctionDocumentation => {
                if let Some(target) = &context.target_module {
                    prompt.push_str(&format!(
                        "Generate comprehensive documentation for the function '{}'. Include:\n",
                        target.name
                    ));
                    prompt.push_str("- What the function does\n");
                    prompt.push_str("- Parameter explanations\n");
                    prompt.push_str("- Return value description\n");
                    prompt.push_str("- Usage examples\n");
                    prompt.push_str("- Error conditions\n");
                }
            }
            EnhancementType::UsageExamples => {
                prompt.push_str("Generate practical usage examples showing how to use this code. Include:\n");
                prompt.push_str("- Basic usage patterns\n");
                prompt.push_str("- Common use cases\n");
                prompt.push_str("- Edge cases to be aware of\n");
                prompt.push_str("- Integration examples\n");
            }
            EnhancementType::ArchitecturalInsights => {
                prompt.push_str("Explain the architectural decisions and design patterns in this code. Focus on:\n");
                prompt.push_str("- Why this approach was chosen\n");
                prompt.push_str("- Trade-offs and alternatives\n");
                prompt.push_str("- Scalability considerations\n");
                prompt.push_str("- Maintainability aspects\n");
            }
            EnhancementType::ImplementationDetails => {
                prompt.push_str("Provide implementation details that would help maintainers. Include:\n");
                prompt.push_str("- Performance considerations\n");
                prompt.push_str("- Error handling strategies\n");
                prompt.push_str("- Resource management\n");
                prompt.push_str("- Thread safety (if applicable)\n");
            }
            EnhancementType::TestingStrategy => {
                prompt.push_str("Suggest a testing strategy for this code. Include:\n");
                prompt.push_str("- Unit test approaches\n");
                prompt.push_str("- Integration test scenarios\n");
                prompt.push_str("- Mock strategies\n");
                prompt.push_str("- Performance test requirements\n");
            }
            EnhancementType::Custom(instruction) => {
                prompt.push_str(instruction);
                prompt.push_str("\n");
            }
        }

        prompt.push_str("\nProvide clear, professional documentation that would be helpful for both new team members and experienced developers. Keep explanations concise but comprehensive.");

        prompt
    }
}

#[async_trait]
impl LlmDocumenter for RigProvider {
    async fn enhance_documentation(&self, request: EnhancementRequest) -> Result<EnhancementResponse> {
        let prompt = self.build_enhancement_prompt(&request);

        match self.config.provider.as_str() {
            "rig-openai" => self.call_openai_api(&prompt).await,
            _ => {
                // Placeholder for other Rig providers
                Ok(EnhancementResponse {
                    content: format!("Enhanced documentation for {} (placeholder - provider not yet implemented)",
                                     request.context.file.path.display()),
                    confidence: Some(0.5),
                    suggestions: vec!["Implement actual LLM provider".to_string()],
                    metadata: HashMap::new(),
                })
            }
        }
    }

    fn provider_name(&self) -> &str {
        match self.config.provider.as_str() {
            "rig-openai" => "OpenAI via Rig",
            "rig-anthropic" => "Anthropic via Rig",
            "rig-google" => "Google via Rig",
            "rig-ollama" => "Ollama via Rig",
            _ => "Unknown Rig Provider",
        }
    }

    fn model_name(&self) -> &str {
        &self.config.model
    }

    async fn health_check(&self) -> Result<bool> {
        // TODO: Implement actual health check
        Ok(true)
    }

    fn capabilities(&self) -> DocumenterCapabilities {
        DocumenterCapabilities {
            max_context_tokens: Some(8000),
            max_response_tokens: self.config.max_tokens,
            supported_enhancements: vec![
                EnhancementType::ModuleOverview,
                EnhancementType::FunctionDocumentation,
                EnhancementType::UsageExamples,
                EnhancementType::ArchitecturalInsights,
                EnhancementType::ImplementationDetails,
                EnhancementType::TestingStrategy,
            ],
            supports_streaming: false,
            supports_code_analysis: true,
        }
    }
}

/// Cortex API provider (work-specific)
pub struct CortexProvider {
    config: LlmConfig,
    client: reqwest::Client,
}

impl CortexProvider {
    pub fn new(config: &LlmConfig) -> Result<Self> {
        if config.base_url.is_none() {
            return Err(CodesworthError::Config(
                "Base URL required for Cortex API".to_string()
            ));
        }

        Ok(Self {
            config: config.clone(),
            client: reqwest::Client::new(),
        })
    }
}

#[async_trait]
impl LlmDocumenter for CortexProvider {
    async fn enhance_documentation(&self, request: EnhancementRequest) -> Result<EnhancementResponse> {
        // TODO: Implement actual Cortex API integration
        // This is a stub for work implementation

        let base_url = self.config.base_url.as_ref().unwrap();
        let prompt = format!("Enhance documentation: {:?}", request.enhancement_type);

        // Placeholder API call structure
        let payload = json!({
            "model": self.config.model,
            "prompt": prompt,
            "max_tokens": self.config.max_tokens.unwrap_or(2000),
            "temperature": self.config.temperature.unwrap_or(0.3)
        });

        // TODO: Implement actual HTTP request
        // let response = self.client
        //     .post(&format!("{}/v1/generate", base_url))
        //     .json(&payload)
        //     .send()
        //     .await?;

        // Placeholder response
        Ok(EnhancementResponse {
            content: format!("Cortex-enhanced documentation (placeholder) for {}",
                             request.context.file.path.display()),
            confidence: Some(0.9),
            suggestions: vec!["Add more specific examples".to_string()],
            metadata: {
                let mut map = HashMap::new();
                map.insert("provider".to_string(), "Cortex API".to_string());
                map.insert("endpoint".to_string(), base_url.clone());
                map
            },
        })
    }

    fn provider_name(&self) -> &str {
        match self.config.provider.as_str() {
            "cortex-gemini" => "Cortex Gemini",
            "cortex-claude" => "Cortex Claude",
            _ => "Cortex API",
        }
    }

    fn model_name(&self) -> &str {
        &self.config.model
    }

    async fn health_check(&self) -> Result<bool> {
        // TODO: Implement Cortex health check
        Ok(true)
    }

    fn capabilities(&self) -> DocumenterCapabilities {
        DocumenterCapabilities::default()
    }
}