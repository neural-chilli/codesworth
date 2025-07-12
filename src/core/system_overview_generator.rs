// src/core/system_overview_generator.rs
use std::path::Path;
use serde::{Serialize, Deserialize};

use crate::error::Result;
use super::{
    PackageAnalysis, HumanContext, SystemContext, LlmDocumenter,
    hierarchical_analyzer::{HierarchicalAnalyzer, SystemUnderstanding}
};

/// Generates comprehensive system-level documentation
pub struct SystemOverviewGenerator {
    hierarchical_analyzer: HierarchicalAnalyzer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemOverview {
    /// Executive summary of what the system does
    pub executive_summary: String,

    /// Business domain and purpose
    pub business_context: BusinessContext,

    /// User personas and workflows
    pub user_guide: UserGuide,

    /// Technical architecture overview
    pub technical_overview: TechnicalOverview,

    /// Package organization and relationships
    pub package_architecture: PackageArchitecture,

    /// Key insights and gotchas
    pub system_insights: Vec<String>,

    /// Getting started guide
    pub getting_started: GettingStarted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessContext {
    /// What industry/domain this serves
    pub domain: String,

    /// Primary business problem being solved
    pub problem_statement: String,

    /// Target users and their goals
    pub target_users: Vec<TargetUser>,

    /// Core business value proposition
    pub value_proposition: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetUser {
    pub persona: String,
    pub goals: Vec<String>,
    pub typical_workflows: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserGuide {
    /// Main user workflows with step-by-step guidance
    pub primary_workflows: Vec<WorkflowGuide>,

    /// Common use cases and examples
    pub use_cases: Vec<UseCase>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowGuide {
    pub name: String,
    pub description: String,
    pub steps: Vec<WorkflowStep>,
    pub technical_notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub step_number: usize,
    pub user_action: String,
    pub system_response: String,
    pub business_rules: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UseCase {
    pub title: String,
    pub scenario: String,
    pub expected_outcome: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechnicalOverview {
    /// High-level architecture pattern
    pub architecture_pattern: String,

    /// Technology stack summary
    pub technology_stack: TechStack,

    /// Key architectural decisions
    pub architectural_decisions: Vec<ArchitecturalDecision>,

    /// Performance and scalability characteristics
    pub non_functional_requirements: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechStack {
    pub languages: Vec<String>,
    pub frameworks: Vec<String>,
    pub databases: Vec<String>,
    pub external_services: Vec<String>,
    pub deployment: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitecturalDecision {
    pub decision: String,
    pub rationale: String,
    pub alternatives_considered: Vec<String>,
    pub trade_offs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageArchitecture {
    /// Package organization strategy
    pub organization_strategy: String,

    /// Package relationships and dependencies
    pub package_relationships: Vec<PackageRelationship>,

    /// Architectural layers
    pub architectural_layers: Vec<ArchitecturalLayer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageRelationship {
    pub from_package: String,
    pub to_package: String,
    pub relationship_type: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitecturalLayer {
    pub layer_name: String,
    pub purpose: String,
    pub packages: Vec<String>,
    pub responsibilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GettingStarted {
    /// Prerequisites for running the system
    pub prerequisites: Vec<String>,

    /// Setup and installation steps
    pub setup_steps: Vec<SetupStep>,

    /// Configuration requirements
    pub configuration: Vec<ConfigurationItem>,

    /// First steps for new users
    pub first_steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupStep {
    pub step_number: usize,
    pub title: String,
    pub instructions: String,
    pub verification: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurationItem {
    pub item: String,
    pub description: String,
    pub required: bool,
    pub default_value: Option<String>,
}

impl SystemOverviewGenerator {
    pub fn new(context_window_limit: usize) -> Self {
        Self {
            hierarchical_analyzer: HierarchicalAnalyzer::new(context_window_limit),
        }
    }

    /// Generate comprehensive system overview
    pub async fn generate_system_overview(
        &self,
        packages: &[PackageAnalysis],
        human_context: &HumanContext,
        system_context: &SystemContext,
        llm_documenter: &dyn LlmDocumenter,
    ) -> Result<SystemOverview> {

        // Step 1: Build comprehensive system understanding
        let system_understanding = self.hierarchical_analyzer.analyze_full_system(
            packages,
            human_context,
            system_context,
            llm_documenter,
        ).await?;

        // Step 2: Generate each section of the overview
        let business_context = self.generate_business_context(&system_understanding, llm_documenter).await?;
        let user_guide = self.generate_user_guide(&system_understanding, llm_documenter).await?;
        let technical_overview = self.generate_technical_overview(&system_understanding, packages, llm_documenter).await?;
        let package_architecture = self.generate_package_architecture(packages, &system_understanding).await?;
        let getting_started = self.generate_getting_started(packages, human_context, llm_documenter).await?;

        // Step 3: Generate executive summary
        let executive_summary = self.generate_executive_summary(
            &business_context,
            &user_guide,
            &technical_overview,
            llm_documenter,
        ).await?;

        Ok(SystemOverview {
            executive_summary,
            business_context,
            user_guide,
            technical_overview,
            package_architecture,
            system_insights: system_understanding.system_insights.iter()
                .map(|insight| insight.insight.clone())
                .collect(),
            getting_started,
        })
    }

    /// Generate business context section
    async fn generate_business_context(
        &self,
        understanding: &SystemUnderstanding,
        llm_documenter: &dyn LlmDocumenter,
    ) -> Result<BusinessContext> {

        let prompt = format!(
            "Based on this system analysis, create a business context overview:\n\n\
            System Purpose: {}\n\
            Business Domain: {:?}\n\
            User Workflows: {:?}\n\n\
            Generate a business context that explains:\n\
            1. What industry/domain this system serves\n\
            2. The primary business problem being solved\n\
            3. Target users and their goals\n\
            4. Core business value proposition\n\n\
            Respond in JSON format:\n\
            {{\n\
              \"domain\": \"Industry/domain name\",\n\
              \"problem_statement\": \"Business problem being solved\",\n\
              \"target_users\": [\n\
                {{\n\
                  \"persona\": \"User type\",\n\
                  \"goals\": [\"Goal 1\", \"Goal 2\"],\n\
                  \"typical_workflows\": [\"Workflow 1\", \"Workflow 2\"]\n\
                }}\n\
              ],\n\
              \"value_proposition\": \"Core value delivered\"\n\
            }}",
            understanding.system_purpose,
            understanding.business_domain,
            understanding.user_workflows
        );

        let response = self.hierarchical_analyzer.make_chunked_llm_call(prompt, llm_documenter).await?;

        // Parse JSON response or provide fallback
        serde_json::from_str(&response.content).unwrap_or_else(|_| BusinessContext {
            domain: understanding.business_domain.domain_type.clone(),
            problem_statement: format!("Provides {} functionality", understanding.business_domain.subdomain),
            target_users: vec![TargetUser {
                persona: "End Users".to_string(),
                goals: vec!["Use the system effectively".to_string()],
                typical_workflows: understanding.user_workflows.iter()
                    .map(|w| w.name.clone())
                    .collect(),
            }],
            value_proposition: understanding.system_purpose.clone(),
        })
    }

    /// Generate user guide section
    async fn generate_user_guide(
        &self,
        understanding: &SystemUnderstanding,
        llm_documenter: &dyn LlmDocumenter,
    ) -> Result<UserGuide> {

        let workflows: Vec<WorkflowGuide> = understanding.user_workflows.iter()
            .map(|workflow| WorkflowGuide {
                name: workflow.name.clone(),
                description: format!("How to complete {}", workflow.name.to_lowercase()),
                steps: workflow.steps.iter().enumerate()
                    .map(|(i, step)| WorkflowStep {
                        step_number: i + 1,
                        user_action: step.description.clone(),
                        system_response: step.technical_implementation.clone(),
                        business_rules: step.business_rules.clone(),
                    })
                    .collect(),
                technical_notes: format!("Involves packages: {}", workflow.involved_packages.join(", ")),
            })
            .collect();

        // Generate use cases based on workflows
        let use_cases: Vec<UseCase> = workflows.iter()
            .map(|workflow| UseCase {
                title: format!("Typical {} Scenario", workflow.name),
                scenario: format!("User wants to {}", workflow.description.to_lowercase()),
                expected_outcome: format!("Successfully completed {}", workflow.name.to_lowercase()),
            })
            .collect();

        Ok(UserGuide {
            primary_workflows: workflows,
            use_cases,
        })
    }

    /// Generate technical overview section
    async fn generate_technical_overview(
        &self,
        understanding: &SystemUnderstanding,
        packages: &[PackageAnalysis],
        llm_documenter: &dyn LlmDocumenter,
    ) -> Result<TechnicalOverview> {

        // Analyze technology stack from dependencies
        let tech_stack = self.analyze_technology_stack(packages);

        Ok(TechnicalOverview {
            architecture_pattern: understanding.architecture_overview.pattern.clone(),
            technology_stack: tech_stack,
            architectural_decisions: understanding.architecture_overview.key_decisions.iter()
                .map(|decision| ArchitecturalDecision {
                    decision: decision.title.clone(),
                    rationale: decision.decision.clone(),
                    alternatives_considered: vec![], // TODO: Extract from ADRs
                    trade_offs: vec![], // TODO: Extract from consequences
                })
                .collect(),
            non_functional_requirements: understanding.architecture_overview.non_functional_attributes.clone(),
        })
    }

    /// Generate package architecture section
    async fn generate_package_architecture(
        &self,
        packages: &[PackageAnalysis],
        understanding: &SystemUnderstanding,
    ) -> Result<PackageArchitecture> {

        // Analyze package relationships
        let mut relationships = Vec::new();
        for package in packages {
            for dep in &package.dependencies.internal_deps {
                relationships.push(PackageRelationship {
                    from_package: package.package_name.clone(),
                    to_package: dep.package_name.clone(),
                    relationship_type: format!("{:?}", dep.relationship_type),
                    description: format!("Package {} depends on {}", package.package_name, dep.package_name),
                });
            }
        }

        // Identify architectural layers
        let layers = self.identify_architectural_layers(packages);

        Ok(PackageArchitecture {
            organization_strategy: "Domain-driven package organization".to_string(),
            package_relationships: relationships,
            architectural_layers: layers,
        })
    }

    /// Generate getting started guide
    async fn generate_getting_started(
        &self,
        packages: &[PackageAnalysis],
        human_context: &HumanContext,
        llm_documenter: &dyn LlmDocumenter,
    ) -> Result<GettingStarted> {

        // Analyze prerequisites from dependencies and configuration
        let prerequisites = self.analyze_prerequisites(packages, human_context);

        // Generate setup steps
        let setup_steps = vec![
            SetupStep {
                step_number: 1,
                title: "Install Prerequisites".to_string(),
                instructions: "Install required runtime and dependencies".to_string(),
                verification: "Verify installations are working".to_string(),
            },
            SetupStep {
                step_number: 2,
                title: "Clone and Build".to_string(),
                instructions: "Clone repository and build the application".to_string(),
                verification: "Ensure build completes successfully".to_string(),
            },
            SetupStep {
                step_number: 3,
                title: "Configure Application".to_string(),
                instructions: "Set up required configuration files".to_string(),
                verification: "Validate configuration is correct".to_string(),
            },
        ];

        Ok(GettingStarted {
            prerequisites,
            setup_steps,
            configuration: vec![], // TODO: Extract from config files
            first_steps: vec![
                "Start the application".to_string(),
                "Access the user interface".to_string(),
                "Try the main workflows".to_string(),
            ],
        })
    }

    /// Generate executive summary
    async fn generate_executive_summary(
        &self,
        business_context: &BusinessContext,
        user_guide: &UserGuide,
        technical_overview: &TechnicalOverview,
        llm_documenter: &dyn LlmDocumenter,
    ) -> Result<String> {

        let prompt = format!(
            "Create a compelling executive summary for this system:\n\n\
            Business Context: {:?}\n\
            User Workflows: {:?}\n\
            Technology: {:?}\n\n\
            Write a 2-3 paragraph executive summary that:\n\
            1. Clearly states what this system does and why it exists\n\
            2. Explains the key business value it provides\n\
            3. Gives a high-level technical overview\n\
            4. Mentions the primary user workflows\n\n\
            Make it engaging and informative for both technical and business audiences.",
            business_context,
            user_guide.primary_workflows,
            technical_overview.technology_stack
        );

        let response = self.hierarchical_analyzer.make_chunked_llm_call(prompt, llm_documenter).await?;

        Ok(response.content)
    }

    /// Write system overview to markdown file
    pub async fn write_system_overview(&self, overview: &SystemOverview, output_path: &Path) -> Result<()> {
        let markdown = self.format_as_markdown(overview);
        std::fs::write(output_path, markdown)?;
        Ok(())
    }

    /// Format system overview as comprehensive markdown
    fn format_as_markdown(&self, overview: &SystemOverview) -> String {
        let mut md = String::new();

        // Title and executive summary
        md.push_str("# System Overview\n\n");
        md.push_str("## Executive Summary\n\n");
        md.push_str(&overview.executive_summary);
        md.push_str("\n\n");

        // Business context
        md.push_str("## Business Context\n\n");
        md.push_str(&format!("**Domain**: {}\n\n", overview.business_context.domain));
        md.push_str(&format!("**Problem Statement**: {}\n\n", overview.business_context.problem_statement));
        md.push_str(&format!("**Value Proposition**: {}\n\n", overview.business_context.value_proposition));

        md.push_str("### Target Users\n\n");
        for user in &overview.business_context.target_users {
            md.push_str(&format!("**{}**:\n", user.persona));
            md.push_str(&format!("- Goals: {}\n", user.goals.join(", ")));
            md.push_str(&format!("- Workflows: {}\n\n", user.typical_workflows.join(", ")));
        }

        // User guide
        md.push_str("## How to Use This System\n\n");
        for workflow in &overview.user_guide.primary_workflows {
            md.push_str(&format!("### {}\n\n", workflow.name));
            md.push_str(&format!("{}\n\n", workflow.description));

            for step in &workflow.steps {
                md.push_str(&format!("{}. **{}** → {}\n",
                                     step.step_number, step.user_action, step.system_response));
                if !step.business_rules.is_empty() {
                    md.push_str(&format!("   - Rules: {}\n", step.business_rules.join(", ")));
                }
            }
            md.push_str("\n");
        }

        // Technical overview
        md.push_str("## Technical Overview\n\n");
        md.push_str(&format!("**Architecture Pattern**: {}\n\n", overview.technical_overview.architecture_pattern));

        md.push_str("### Technology Stack\n\n");
        let tech = &overview.technical_overview.technology_stack;
        md.push_str(&format!("- **Languages**: {}\n", tech.languages.join(", ")));
        md.push_str(&format!("- **Frameworks**: {}\n", tech.frameworks.join(", ")));
        if !tech.databases.is_empty() {
            md.push_str(&format!("- **Databases**: {}\n", tech.databases.join(", ")));
        }
        if !tech.external_services.is_empty() {
            md.push_str(&format!("- **External Services**: {}\n", tech.external_services.join(", ")));
        }
        md.push_str("\n");

        // Package architecture
        md.push_str("## Package Architecture\n\n");
        md.push_str(&format!("**Organization Strategy**: {}\n\n", overview.package_architecture.organization_strategy));

        md.push_str("### Architectural Layers\n\n");
        for layer in &overview.package_architecture.architectural_layers {
            md.push_str(&format!("**{}**: {}\n", layer.layer_name, layer.purpose));
            md.push_str(&format!("- Packages: {}\n", layer.packages.join(", ")));
            md.push_str(&format!("- Responsibilities: {}\n\n", layer.responsibilities.join(", ")));
        }

        // System insights
        if !overview.system_insights.is_empty() {
            md.push_str("## Key Insights\n\n");
            for insight in &overview.system_insights {
                md.push_str(&format!("- {}\n", insight));
            }
            md.push_str("\n");
        }

        // Getting started
        md.push_str("## Getting Started\n\n");
        md.push_str("### Prerequisites\n\n");
        for prereq in &overview.getting_started.prerequisites {
            md.push_str(&format!("- {}\n", prereq));
        }
        md.push_str("\n");

        md.push_str("### Setup Steps\n\n");
        for step in &overview.getting_started.setup_steps {
            md.push_str(&format!("{}. **{}**\n", step.step_number, step.title));
            md.push_str(&format!("   {}\n", step.instructions));
            md.push_str(&format!("   *Verification*: {}\n\n", step.verification));
        }

        md.push_str("### First Steps\n\n");
        for (i, step) in overview.getting_started.first_steps.iter().enumerate() {
            md.push_str(&format!("{}. {}\n", i + 1, step));
        }
        md.push_str("\n");

        // Package links
        md.push_str("## Package Documentation\n\n");
        md.push_str("For detailed information about specific packages, see:\n\n");
        // This would be populated with actual package links
        md.push_str("- [Package documentation links will be added here]\n\n");

        md.push_str("---\n\n*This system overview was generated by Codesworth's hierarchical analysis system.*\n");

        md
    }

    // Helper methods

    fn analyze_technology_stack(&self, packages: &[PackageAnalysis]) -> TechStack {
        let mut languages = std::collections::HashSet::new();
        let mut frameworks = std::collections::HashSet::new();
        let mut databases = std::collections::HashSet::new();
        let mut external_services = std::collections::HashSet::new();

        for package in packages {
            // Collect languages
            for file in &package.files {
                languages.insert(file.language.clone());
            }

            // Analyze dependencies for frameworks and services
            for dep in &package.dependencies.external_deps {
                match dep.usage_context.to_lowercase().as_str() {
                    s if s.contains("framework") => { frameworks.insert(dep.name.clone()); }
                    s if s.contains("database") => { databases.insert(dep.name.clone()); }
                    s if s.contains("service") => { external_services.insert(dep.name.clone()); }
                    _ => { frameworks.insert(dep.name.clone()); }
                }
            }
        }

        TechStack {
            languages: languages.into_iter().collect(),
            frameworks: frameworks.into_iter().collect(),
            databases: databases.into_iter().collect(),
            external_services: external_services.into_iter().collect(),
            deployment: vec![], // TODO: Detect from Docker, K8s files, etc.
        }
    }

    fn identify_architectural_layers(&self, packages: &[PackageAnalysis]) -> Vec<ArchitecturalLayer> {
        let mut layers = Vec::new();

        // Group packages by common patterns
        let mut controller_packages = Vec::new();
        let mut service_packages = Vec::new();
        let mut domain_packages = Vec::new();
        let mut config_packages = Vec::new();
        let mut util_packages = Vec::new();

        for package in packages {
            match package.package_name.to_lowercase().as_str() {
                s if s.contains("controller") || s.contains("handler") || s.contains("endpoint") => {
                    controller_packages.push(package.package_name.clone());
                }
                s if s.contains("service") || s.contains("business") => {
                    service_packages.push(package.package_name.clone());
                }
                s if s.contains("domain") || s.contains("model") || s.contains("entity") => {
                    domain_packages.push(package.package_name.clone());
                }
                s if s.contains("config") || s.contains("configuration") => {
                    config_packages.push(package.package_name.clone());
                }
                s if s.contains("util") || s.contains("helper") || s.contains("common") => {
                    util_packages.push(package.package_name.clone());
                }
                _ => {
                    // Default to service layer
                    service_packages.push(package.package_name.clone());
                }
            }
        }

        if !controller_packages.is_empty() {
            layers.push(ArchitecturalLayer {
                layer_name: "Presentation Layer".to_string(),
                purpose: "Handles user interactions and external interfaces".to_string(),
                packages: controller_packages,
                responsibilities: vec![
                    "HTTP request handling".to_string(),
                    "User input validation".to_string(),
                    "Response formatting".to_string(),
                ],
            });
        }

        if !service_packages.is_empty() {
            layers.push(ArchitecturalLayer {
                layer_name: "Business Logic Layer".to_string(),
                purpose: "Implements core business operations and workflows".to_string(),
                packages: service_packages,
                responsibilities: vec![
                    "Business rule enforcement".to_string(),
                    "Workflow orchestration".to_string(),
                    "Data processing".to_string(),
                ],
            });
        }

        if !domain_packages.is_empty() {
            layers.push(ArchitecturalLayer {
                layer_name: "Domain Model Layer".to_string(),
                purpose: "Defines business entities and domain concepts".to_string(),
                packages: domain_packages,
                responsibilities: vec![
                    "Business entity definitions".to_string(),
                    "Domain logic encapsulation".to_string(),
                    "Data contracts".to_string(),
                ],
            });
        }

        if !config_packages.is_empty() {
            layers.push(ArchitecturalLayer {
                layer_name: "Configuration Layer".to_string(),
                purpose: "Manages application configuration and setup".to_string(),
                packages: config_packages,
                responsibilities: vec![
                    "Application configuration".to_string(),
                    "Dependency injection".to_string(),
                    "Environment setup".to_string(),
                ],
            });
        }

        if !util_packages.is_empty() {
            layers.push(ArchitecturalLayer {
                layer_name: "Utility Layer".to_string(),
                purpose: "Provides common utilities and helper functions".to_string(),
                packages: util_packages,
                responsibilities: vec![
                    "Common utilities".to_string(),
                    "Helper functions".to_string(),
                    "Shared logic".to_string(),
                ],
            });
        }

        layers
    }

    fn analyze_prerequisites(&self, packages: &[PackageAnalysis], human_context: &HumanContext) -> Vec<String> {
        let mut prerequisites = Vec::new();

        // Analyze from dependencies and configuration hints
        for hint in &human_context.configuration_hints {
            match hint.source.as_str() {
                "pom.xml" | "build.gradle" => {
                    prerequisites.push("Java JDK 11 or higher".to_string());
                    prerequisites.push("Maven or Gradle build tool".to_string());
                }
                "package.json" => {
                    prerequisites.push("Node.js 16 or higher".to_string());
                    prerequisites.push("npm or yarn package manager".to_string());
                }
                "Cargo.toml" => {
                    prerequisites.push("Rust toolchain".to_string());
                    prerequisites.push("Cargo package manager".to_string());
                }
                "requirements.txt" | "pyproject.toml" => {
                    prerequisites.push("Python 3.8 or higher".to_string());
                    prerequisites.push("pip package manager".to_string());
                }
                _ => {}
            }
        }

        // Add common prerequisites based on detected technologies
        for package in packages {
            for dep in &package.dependencies.external_deps {
                if dep.usage_context.to_lowercase().contains("database") {
                    prerequisites.push("Database server (PostgreSQL, MySQL, etc.)".to_string());
                    break;
                }
            }
        }

        prerequisites.sort();
        prerequisites.dedup();
        prerequisites
    }
}

impl Default for SystemOverviewGenerator {
    fn default() -> Self {
        Self::new(8000) // Default context window
    }
}