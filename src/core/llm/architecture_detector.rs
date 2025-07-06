use std::path::Path;
use super::documenter::ArchitectureDocs;
use crate::error::Result;

/// Detects architectural context from project files
pub struct ArchitectureDetector;

impl ArchitectureDetector {
    pub fn new() -> Self {
        Self
    }

    /// Extract architecture context from project documentation and code
    pub async fn detect_architecture(&self, project_root: &Path) -> Result<Option<ArchitectureDocs>> {
        let mut arch_docs = ArchitectureDocs {
            system_overview: None,
            architectural_decisions: Vec::new(),
            technology_stack: Vec::new(),
            design_patterns: Vec::new(),
            integrations: Vec::new(),
        };

        // Look for README files
        if let Some(overview) = self.extract_from_readme(project_root).await? {
            arch_docs.system_overview = Some(overview);
        }

        // Detect technology stack from build files
        arch_docs.technology_stack = self.detect_technology_stack(project_root).await?;

        // Look for architectural decision records
        arch_docs.architectural_decisions = self.extract_architectural_decisions(project_root).await?;

        // Detect common integrations
        arch_docs.integrations = self.detect_integrations(project_root).await?;

        // Detect design patterns from code structure
        arch_docs.design_patterns = self.detect_design_patterns(project_root).await?;

        // Return None if we didn't find any useful context
        if arch_docs.system_overview.is_none()
            && arch_docs.technology_stack.is_empty()
            && arch_docs.architectural_decisions.is_empty() {
            Ok(None)
        } else {
            Ok(Some(arch_docs))
        }
    }

    async fn extract_from_readme(&self, project_root: &Path) -> Result<Option<String>> {
        let readme_candidates = ["README.md", "readme.md", "README.txt", "README"];

        for candidate in &readme_candidates {
            let readme_path = project_root.join(candidate);
            if readme_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&readme_path) {
                    // Extract first few paragraphs as system overview
                    let lines: Vec<&str> = content.lines().collect();
                    let mut overview_lines = Vec::new();
                    let mut in_content = false;

                    for line in lines {
                        let trimmed = line.trim();

                        // Skip title lines
                        if trimmed.starts_with('#') && !in_content {
                            continue;
                        }

                        // Stop at installation, usage, or setup sections
                        if trimmed.to_lowercase().contains("installation")
                            || trimmed.to_lowercase().contains("usage")
                            || trimmed.to_lowercase().contains("getting started") {
                            break;
                        }

                        if !trimmed.is_empty() {
                            in_content = true;
                            overview_lines.push(trimmed);

                            // Limit to first few meaningful lines
                            if overview_lines.len() > 5 {
                                break;
                            }
                        }
                    }

                    if !overview_lines.is_empty() {
                        return Ok(Some(overview_lines.join(" ")));
                    }
                }
            }
        }

        Ok(None)
    }

    async fn detect_technology_stack(&self, project_root: &Path) -> Result<Vec<String>> {
        let mut stack = Vec::new();

        // Java/Spring detection
        if project_root.join("pom.xml").exists() || project_root.join("build.gradle").exists() {
            stack.push("Java".to_string());

            // Check for Spring
            if self.file_contains_patterns(project_root, &["pom.xml", "build.gradle"], &["spring"]).await? {
                stack.push("Spring Framework".to_string());
            }

            // Check for Spring Boot
            if self.file_contains_patterns(project_root, &["pom.xml", "build.gradle"], &["spring-boot"]).await? {
                stack.push("Spring Boot".to_string());
            }
        }

        // Python detection
        if project_root.join("requirements.txt").exists()
            || project_root.join("pyproject.toml").exists()
            || project_root.join("setup.py").exists() {
            stack.push("Python".to_string());

            // Check for Django
            if self.file_contains_patterns(project_root, &["requirements.txt", "pyproject.toml"], &["django"]).await? {
                stack.push("Django".to_string());
            }

            // Check for Flask
            if self.file_contains_patterns(project_root, &["requirements.txt", "pyproject.toml"], &["flask"]).await? {
                stack.push("Flask".to_string());
            }
        }

        // JavaScript/Node.js detection
        if project_root.join("package.json").exists() {
            stack.push("JavaScript".to_string());
            stack.push("Node.js".to_string());

            // Check for React
            if self.file_contains_patterns(project_root, &["package.json"], &["react"]).await? {
                stack.push("React".to_string());
            }

            // Check for Express
            if self.file_contains_patterns(project_root, &["package.json"], &["express"]).await? {
                stack.push("Express.js".to_string());
            }
        }

        // .NET detection
        if project_root.join("*.csproj").exists() || project_root.join("*.sln").exists() {
            stack.push("C#".to_string());
            stack.push(".NET".to_string());
        }

        // Rust detection
        if project_root.join("Cargo.toml").exists() {
            stack.push("Rust".to_string());

            // Check for web frameworks
            if self.file_contains_patterns(project_root, &["Cargo.toml"], &["axum", "warp", "actix"]).await? {
                stack.push("Web Framework".to_string());
            }
        }

        // Database detection
        if self.file_contains_patterns(project_root, &["**/*.sql", "migrations/**/*"], &[""]).await? {
            stack.push("SQL Database".to_string());
        }

        Ok(stack)
    }

    async fn detect_integrations(&self, project_root: &Path) -> Result<Vec<String>> {
        let mut integrations = Vec::new();

        // AI/ML integrations
        if self.file_contains_patterns(project_root, &["**/*.java", "**/*.py", "**/*.js"], &["openai", "anthropic"]).await? {
            integrations.push("LLM Integration (OpenAI/Anthropic)".to_string());
        }

        if self.file_contains_patterns(project_root, &["**/*.java"], &["spring-ai"]).await? {
            integrations.push("Spring AI".to_string());
        }

        // Database integrations
        if self.file_contains_patterns(project_root, &["**/*.java"], &["@Entity", "JpaRepository"]).await? {
            integrations.push("JPA/Hibernate".to_string());
        }

        // Message queues
        if self.file_contains_patterns(project_root, &["**/*"], &["rabbitmq", "kafka", "redis"]).await? {
            integrations.push("Message Queue".to_string());
        }

        // Cloud services
        if self.file_contains_patterns(project_root, &["**/*"], &["aws", "azure", "gcp"]).await? {
            integrations.push("Cloud Services".to_string());
        }

        Ok(integrations)
    }

    async fn detect_design_patterns(&self, project_root: &Path) -> Result<Vec<String>> {
        let mut patterns = Vec::new();

        // Service pattern
        if self.file_contains_patterns(project_root, &["**/*.java"], &["@Service"]).await? {
            patterns.push("Service Pattern".to_string());
        }

        // Repository pattern
        if self.file_contains_patterns(project_root, &["**/*.java"], &["@Repository", "Repository"]).await? {
            patterns.push("Repository Pattern".to_string());
        }

        // Controller pattern
        if self.file_contains_patterns(project_root, &["**/*.java"], &["@Controller", "@RestController"]).await? {
            patterns.push("MVC Pattern".to_string());
        }

        // Builder pattern
        if self.file_contains_patterns(project_root, &["**/*"], &["Builder", ".builder()"]).await? {
            patterns.push("Builder Pattern".to_string());
        }

        // Factory pattern
        if self.file_contains_patterns(project_root, &["**/*"], &["Factory"]).await? {
            patterns.push("Factory Pattern".to_string());
        }

        Ok(patterns)
    }

    async fn extract_architectural_decisions(&self, project_root: &Path) -> Result<Vec<String>> {
        let mut decisions = Vec::new();

        // Look for ADR directories
        let adr_paths = [
            "docs/decisions",
            "docs/adr",
            "architecture/decisions",
            "adr"
        ];

        for adr_path in &adr_paths {
            let full_path = project_root.join(adr_path);
            if full_path.exists() && full_path.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&full_path) {
                    for entry in entries.flatten() {
                        if let Some(file_name) = entry.file_name().to_str() {
                            if file_name.ends_with(".md") {
                                decisions.push(file_name.replace(".md", "").replace("-", " "));
                            }
                        }
                    }
                }
            }
        }

        Ok(decisions)
    }

    async fn file_contains_patterns(&self, project_root: &Path, file_patterns: &[&str], search_patterns: &[&str]) -> Result<bool> {
        // Simple implementation - in a real system you'd want proper glob matching
        for file_pattern in file_patterns {
            let file_path = project_root.join(file_pattern);
            if file_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&file_path) {
                    for pattern in search_patterns {
                        if !pattern.is_empty() && content.contains(pattern) {
                            return Ok(true);
                        }
                    }
                }
            }
        }
        Ok(false)
    }
}