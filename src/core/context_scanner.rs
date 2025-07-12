// src/core/context_scanner.rs
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use regex::Regex;

use crate::error::Result;
use super::batch_processor::{
    HumanContext, ArchitectureDoc, ArchitecturalDecision, ArchitecturalComment,
    ConfigurationHint, CommentCategory, SystemContext, RelatedPackage,
    PatternUsage, ArchitecturalTheme
};

/// Scans project for human-authored documentation and architectural context
pub struct ContextScanner {
    // Regex patterns for extracting structured information
    adr_title_regex: Regex,
    todo_comment_regex: Regex,
    performance_comment_regex: Regex,
    security_comment_regex: Regex,
}

impl ContextScanner {
    pub fn new() -> Result<Self> {
        Ok(Self {
            adr_title_regex: Regex::new(r"^#\s*(?:ADR[-\s]*)?(\d+):\s*(.+)$")?,
            todo_comment_regex: Regex::new(r"(?i)(?:TODO|FIXME|HACK|NOTE):\s*(.+)")?,
            performance_comment_regex: Regex::new(r"(?i)(?:performance|perf|slow|fast|optimization|cache|memory):\s*(.+)")?,
            security_comment_regex: Regex::new(r"(?i)(?:security|auth|permission|crypto|secure):\s*(.+)")?,
        })
    }

    /// Scan project directory for human context
    pub async fn scan_project_context<P: AsRef<Path>>(&self, project_root: P) -> Result<HumanContext> {
        let root = project_root.as_ref();

        let readme_content = self.find_and_read_readme(root).await?;
        let architecture_docs = self.scan_architecture_docs(root).await?;
        let adrs = self.scan_architectural_decisions(root).await?;
        let inline_comments = self.scan_architectural_comments(root).await?;
        let configuration_hints = self.scan_configuration_files(root).await?;

        Ok(HumanContext {
            readme_content,
            architecture_docs,
            adrs,
            inline_comments,
            configuration_hints,
        })
    }

    /// Scan for system-wide context across packages
    pub async fn scan_system_context<P: AsRef<Path>>(&self, project_root: P, package_names: &[String]) -> Result<SystemContext> {
        let root = project_root.as_ref();

        let related_packages = self.analyze_package_relationships(root, package_names).await?;
        let common_patterns = self.detect_common_patterns(root).await?;
        let architectural_themes = self.extract_architectural_themes(root).await?;

        Ok(SystemContext {
            related_packages,
            common_patterns,
            architectural_themes,
        })
    }

    // README scanning

    async fn find_and_read_readme<P: AsRef<Path>>(&self, project_root: P) -> Result<Option<String>> {
        let readme_candidates = [
            "README.md", "readme.md", "README.MD", "Readme.md",
            "README.txt", "readme.txt", "README.rst", "readme.rst",
            "README", "readme"
        ];

        for candidate in &readme_candidates {
            let readme_path = project_root.as_ref().join(candidate);
            if readme_path.exists() {
                match std::fs::read_to_string(&readme_path) {
                    Ok(content) => {
                        // Extract meaningful content, skip badges and boilerplate
                        let cleaned = self.clean_readme_content(&content);
                        if !cleaned.trim().is_empty() {
                            return Ok(Some(cleaned));
                        }
                    }
                    Err(_) => continue,
                }
            }
        }

        Ok(None)
    }

    fn clean_readme_content(&self, content: &str) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let mut cleaned_lines = Vec::new();
        let mut skip_badges = true;

        for line in lines {
            let trimmed = line.trim();

            // Skip badge sections at the top
            if skip_badges && (trimmed.contains("![") || trimmed.contains("[![") || trimmed.is_empty()) {
                if trimmed.starts_with("#") && !trimmed.contains("![") {
                    skip_badges = false;
                    cleaned_lines.push(line);
                }
                continue;
            }
            skip_badges = false;

            // Stop at installation, usage, or build sections for overview
            if trimmed.to_lowercase().starts_with("## installation") ||
                trimmed.to_lowercase().starts_with("## getting started") ||
                trimmed.to_lowercase().starts_with("## usage") ||
                trimmed.to_lowercase().starts_with("## build") {
                break;
            }

            cleaned_lines.push(line);

            // Limit to first few meaningful sections
            if cleaned_lines.len() > 50 {
                break;
            }
        }

        cleaned_lines.join("\n")
    }

    // Architecture documentation scanning

    async fn scan_architecture_docs<P: AsRef<Path>>(&self, project_root: P) -> Result<Vec<ArchitectureDoc>> {
        let mut docs = Vec::new();
        let architecture_paths = [
            "docs/architecture",
            "docs/arch",
            "architecture",
            "docs",
            "doc/architecture"
        ];

        for arch_path in &architecture_paths {
            let full_path = project_root.as_ref().join(arch_path);
            if full_path.exists() && full_path.is_dir() {
                docs.extend(self.scan_docs_directory(&full_path, "architecture").await?);
            }
        }

        // Also look for standalone architecture files
        let arch_files = [
            "ARCHITECTURE.md", "architecture.md", "Architecture.md",
            "DESIGN.md", "design.md", "Design.md",
            "docs/README.md"
        ];

        for arch_file in &arch_files {
            let file_path = project_root.as_ref().join(arch_file);
            if file_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&file_path) {
                    let title = self.extract_title_from_markdown(&content)
                        .unwrap_or_else(|| arch_file.to_string());

                    docs.push(ArchitectureDoc {
                        title,
                        content: self.clean_markdown_content(&content),
                        file_path: arch_file.to_string(),
                        relevance_score: self.calculate_relevance_score(&content),
                    });
                }
            }
        }

        // Sort by relevance
        docs.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap_or(std::cmp::Ordering::Equal));

        Ok(docs)
    }

    async fn scan_docs_directory<P: AsRef<Path>>(&self, docs_dir: P, category: &str) -> Result<Vec<ArchitectureDoc>> {
        let mut docs = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&docs_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && self.is_markdown_file(&path) {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        let title = self.extract_title_from_markdown(&content)
                            .or_else(|| path.file_stem().map(|s| s.to_string_lossy().to_string()))
                            .unwrap_or_else(|| "Untitled".to_string());

                        docs.push(ArchitectureDoc {
                            title,
                            content: self.clean_markdown_content(&content),
                            file_path: path.to_string_lossy().to_string(),
                            relevance_score: self.calculate_relevance_score(&content),
                        });
                    }
                }
            }
        }

        Ok(docs)
    }

    fn is_markdown_file(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| matches!(ext.to_lowercase().as_str(), "md" | "markdown" | "mkd"))
            .unwrap_or(false)
    }

    fn extract_title_from_markdown(&self, content: &str) -> Option<String> {
        content.lines()
            .find(|line| line.starts_with("# "))
            .map(|line| line.trim_start_matches("# ").trim().to_string())
    }

    fn clean_markdown_content(&self, content: &str) -> String {
        // Remove excessive whitespace and normalize
        let lines: Vec<&str> = content.lines()
            .map(|line| line.trim_end())
            .filter(|line| !line.trim().is_empty() || !line.is_empty()) // Keep structure but remove pure whitespace
            .collect();

        lines.join("\n")
    }

    fn calculate_relevance_score(&self, content: &str) -> f32 {
        let mut score = 0.0;
        let content_lower = content.to_lowercase();

        // Architecture-relevant keywords
        let arch_keywords = [
            "architecture", "design", "pattern", "principle", "decision",
            "component", "service", "module", "layer", "interface",
            "microservice", "monolith", "database", "api", "integration"
        ];

        for keyword in &arch_keywords {
            score += content_lower.matches(keyword).count() as f32 * 0.1;
        }

        // Boost for structured content
        if content.contains("## ") { score += 0.5; }
        if content.contains("### ") { score += 0.3; }
        if content.contains("```") { score += 0.2; }

        // Length bonus (but not too much)
        let length_score = (content.len() as f32 / 1000.0).min(2.0);
        score += length_score;

        score.min(10.0)
    }

    // ADR scanning

    async fn scan_architectural_decisions<P: AsRef<Path>>(&self, project_root: P) -> Result<Vec<ArchitecturalDecision>> {
        let mut adrs = Vec::new();
        let adr_paths = [
            "docs/decisions", "docs/adr", "docs/adrs",
            "architecture/decisions", "architecture/adr",
            "adr", "adrs", "decisions"
        ];

        for adr_path in &adr_paths {
            let full_path = project_root.as_ref().join(adr_path);
            if full_path.exists() && full_path.is_dir() {
                adrs.extend(self.scan_adr_directory(&full_path).await?);
            }
        }

        // Look for ADR-style documents in docs/
        let docs_path = project_root.as_ref().join("docs");
        if docs_path.exists() {
            adrs.extend(self.scan_for_adr_documents(&docs_path).await?);
        }

        Ok(adrs)
    }

    async fn scan_adr_directory<P: AsRef<Path>>(&self, adr_dir: P) -> Result<Vec<ArchitecturalDecision>> {
        let mut adrs = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&adr_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && self.is_markdown_file(&path) {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if let Some(adr) = self.parse_adr_document(&content) {
                            adrs.push(adr);
                        }
                    }
                }
            }
        }

        Ok(adrs)
    }

    async fn scan_for_adr_documents<P: AsRef<Path>>(&self, docs_dir: P) -> Result<Vec<ArchitecturalDecision>> {
        let mut adrs = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&docs_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && self.is_markdown_file(&path) {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        // Check if this looks like an ADR
                        if self.looks_like_adr(&content) {
                            if let Some(adr) = self.parse_adr_document(&content) {
                                adrs.push(adr);
                            }
                        }
                    }
                }
            }
        }

        Ok(adrs)
    }

    fn looks_like_adr(&self, content: &str) -> bool {
        let content_lower = content.to_lowercase();
        (content_lower.contains("decision") || content_lower.contains("adr")) &&
            (content_lower.contains("context") || content_lower.contains("status")) &&
            (content_lower.contains("consequence") || content_lower.contains("rationale"))
    }

    fn parse_adr_document(&self, content: &str) -> Option<ArchitecturalDecision> {
        let lines: Vec<&str> = content.lines().collect();

        let mut title = None;
        let mut status = "Unknown".to_string();
        let mut context = String::new();
        let mut decision = String::new();
        let mut consequences = String::new();
        let mut date = None;

        let mut current_section = "";

        for line in lines {
            let trimmed = line.trim();

            // Extract title from first heading
            if title.is_none() && trimmed.starts_with("# ") {
                title = Some(trimmed.trim_start_matches("# ").to_string());
                continue;
            }

            // Look for status
            if trimmed.to_lowercase().starts_with("**status") || trimmed.to_lowercase().starts_with("status:") {
                status = self.extract_adr_status(trimmed);
                continue;
            }

            // Look for date
            if trimmed.to_lowercase().starts_with("**date") || trimmed.to_lowercase().starts_with("date:") {
                date = self.extract_adr_date(trimmed);
                continue;
            }

            // Section headers
            if trimmed.starts_with("## ") {
                let section_name = trimmed.trim_start_matches("## ").to_lowercase();
                current_section = if section_name.contains("context") {
                    "context"
                } else if section_name.contains("decision") {
                    "decision"
                } else if section_name.contains("consequence") {
                    "consequences"
                } else {
                    ""
                };
                continue;
            }

            // Collect content for current section
            if !trimmed.is_empty() {
                match current_section {
                    "context" => {
                        if !context.is_empty() { context.push(' '); }
                        context.push_str(trimmed);
                    }
                    "decision" => {
                        if !decision.is_empty() { decision.push(' '); }
                        decision.push_str(trimmed);
                    }
                    "consequences" => {
                        if !consequences.is_empty() { consequences.push(' '); }
                        consequences.push_str(trimmed);
                    }
                    _ => {}
                }
            }
        }

        if let Some(title) = title {
            Some(ArchitecturalDecision {
                title,
                status,
                context,
                decision,
                consequences,
                date,
            })
        } else {
            None
        }
    }

    fn extract_adr_status(&self, line: &str) -> String {
        let line_lower = line.to_lowercase();
        if line_lower.contains("accepted") { "Accepted".to_string() }
        else if line_lower.contains("proposed") { "Proposed".to_string() }
        else if line_lower.contains("deprecated") { "Deprecated".to_string() }
        else if line_lower.contains("superseded") { "Superseded".to_string() }
        else { "Unknown".to_string() }
    }

    fn extract_adr_date(&self, line: &str) -> Option<String> {
        // Simple date extraction - look for YYYY-MM-DD pattern
        let date_regex = Regex::new(r"\d{4}-\d{2}-\d{2}").ok()?;
        date_regex.find(line).map(|m| m.as_str().to_string())
    }

    // Architectural comments scanning

    async fn scan_architectural_comments<P: AsRef<Path>>(&self, project_root: P) -> Result<Vec<ArchitecturalComment>> {
        let mut comments = Vec::new();

        // Scan source code files for architectural comments
        self.scan_directory_for_comments(project_root.as_ref(), &mut comments).await?;

        // Sort by category importance
        comments.sort_by(|a, b| {
            let a_priority = match a.category {
                CommentCategory::DesignDecision => 5,
                CommentCategory::SecurityNote => 4,
                CommentCategory::PerformanceNote => 3,
                CommentCategory::IntegrationNote => 2,
                CommentCategory::BusinessLogic => 1,
                CommentCategory::TechnicalDebt => 0,
            };
            let b_priority = match b.category {
                CommentCategory::DesignDecision => 5,
                CommentCategory::SecurityNote => 4,
                CommentCategory::PerformanceNote => 3,
                CommentCategory::IntegrationNote => 2,
                CommentCategory::BusinessLogic => 1,
                CommentCategory::TechnicalDebt => 0,
            };
            b_priority.cmp(&a_priority)
        });

        // Limit to most important comments
        comments.truncate(20);

        Ok(comments)
    }

    async fn scan_directory_for_comments<P: AsRef<Path>>(&self, dir: P, comments: &mut Vec<ArchitecturalComment>) -> Result<()> {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    // Skip build directories and common ignore patterns
                    if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                        if matches!(dir_name, "target" | "node_modules" | ".git" | "build" | "dist") {
                            continue;
                        }
                    }
                    Box::pin(self.scan_directory_for_comments(&path, comments)).await?;
                } else if self.is_source_code_file(&path) {
                    self.extract_comments_from_file(&path, comments)?;
                }
            }
        }
        Ok(())
    }

    fn is_source_code_file(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| matches!(ext, "rs" | "java" | "py" | "js" | "ts" | "cs" | "cpp" | "h" | "hpp"))
            .unwrap_or(false)
    }

    fn extract_comments_from_file(&self, file_path: &Path, comments: &mut Vec<ArchitecturalComment>) -> Result<()> {
        let content = std::fs::read_to_string(file_path)?;
        let lines: Vec<&str> = content.lines().collect();

        for (line_number, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Look for various comment patterns that indicate architectural significance
            if let Some(comment) = self.extract_architectural_comment(trimmed, file_path, line_number + 1) {
                comments.push(comment);
            }
        }

        Ok(())
    }

    fn extract_architectural_comment(&self, line: &str, file_path: &Path, line_number: usize) -> Option<ArchitecturalComment> {
        // Skip short or obvious comments
        if line.len() < 20 {
            return None;
        }

        let comment_content = self.extract_comment_content(line)?;

        // Skip if too short after extraction
        if comment_content.len() < 15 {
            return None;
        }

        let category = self.categorize_comment(&comment_content);

        // Only keep architecturally significant comments
        match category {
            CommentCategory::TechnicalDebt if !comment_content.to_lowercase().contains("important") => None,
            _ => Some(ArchitecturalComment {
                content: comment_content,
                file_path: file_path.to_string_lossy().to_string(),
                line_number,
                category,
            })
        }
    }

    fn extract_comment_content(&self, line: &str) -> Option<String> {
        let trimmed = line.trim();

        // Various comment styles
        if trimmed.starts_with("//") {
            Some(trimmed.trim_start_matches("//").trim().to_string())
        } else if trimmed.starts_with("/*") && trimmed.ends_with("*/") {
            Some(trimmed.trim_start_matches("/*").trim_end_matches("*/").trim().to_string())
        } else if trimmed.starts_with("#") {
            Some(trimmed.trim_start_matches("#").trim().to_string())
        } else if trimmed.starts_with("*") && !trimmed.starts_with("*/") {
            Some(trimmed.trim_start_matches("*").trim().to_string())
        } else {
            None
        }
    }

    fn categorize_comment(&self, content: &str) -> CommentCategory {
        let content_lower = content.to_lowercase();

        if self.security_comment_regex.is_match(&content_lower) {
            CommentCategory::SecurityNote
        } else if self.performance_comment_regex.is_match(&content_lower) {
            CommentCategory::PerformanceNote
        } else if content_lower.contains("design") || content_lower.contains("pattern") ||
            content_lower.contains("architecture") || content_lower.contains("chosen") {
            CommentCategory::DesignDecision
        } else if content_lower.contains("integration") || content_lower.contains("api") ||
            content_lower.contains("interface") || content_lower.contains("protocol") {
            CommentCategory::IntegrationNote
        } else if content_lower.contains("business") || content_lower.contains("domain") ||
            content_lower.contains("rule") || content_lower.contains("logic") {
            CommentCategory::BusinessLogic
        } else if self.todo_comment_regex.is_match(content) {
            CommentCategory::TechnicalDebt
        } else {
            CommentCategory::DesignDecision // Default for anything interesting enough to extract
        }
    }

    // Configuration scanning

    async fn scan_configuration_files<P: AsRef<Path>>(&self, project_root: P) -> Result<Vec<ConfigurationHint>> {
        let mut hints = Vec::new();

        // Common configuration files to analyze
        let config_files = [
            ("Cargo.toml", "dependencies"),
            ("package.json", "dependencies"),
            ("pom.xml", "dependencies"),
            ("build.gradle", "dependencies"),
            ("requirements.txt", "dependencies"),
            ("pyproject.toml", "project"),
            ("docker-compose.yml", "services"),
            ("Dockerfile", "deployment"),
            (".env.example", "configuration"),
            ("config.toml", "configuration"),
            ("application.yml", "configuration"),
        ];

        for (file_name, category) in &config_files {
            let file_path = project_root.as_ref().join(file_name);
            if file_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&file_path) {
                    let insights = self.analyze_configuration_file(&content, category);
                    if !insights.is_empty() {
                        hints.push(ConfigurationHint {
                            source: file_name.to_string(),
                            category: category.to_string(),
                            insights,
                        });
                    }
                }
            }
        }

        Ok(hints)
    }

    fn analyze_configuration_file(&self, content: &str, category: &str) -> Vec<String> {
        let mut insights = Vec::new();

        match category {
            "dependencies" => {
                insights.extend(self.analyze_dependencies(content));
            }
            "services" => {
                insights.extend(self.analyze_docker_compose(content));
            }
            "deployment" => {
                insights.extend(self.analyze_dockerfile(content));
            }
            "configuration" | "project" => {
                insights.extend(self.analyze_project_config(content));
            }
            _ => {}
        }

        insights
    }

    fn analyze_dependencies(&self, content: &str) -> Vec<String> {
        let mut insights = Vec::new();

        // Web frameworks
        if content.contains("axum") || content.contains("warp") || content.contains("actix") {
            insights.push("Web service with async HTTP framework".to_string());
        }
        if content.contains("express") || content.contains("fastapi") || content.contains("spring-boot") {
            insights.push("Web service framework".to_string());
        }

        // Databases
        if content.contains("sqlx") || content.contains("diesel") || content.contains("hibernate") {
            insights.push("SQL database integration".to_string());
        }
        if content.contains("redis") || content.contains("mongodb") {
            insights.push("NoSQL database integration".to_string());
        }

        // Async/Concurrency
        if content.contains("tokio") || content.contains("async-std") {
            insights.push("Async runtime for concurrent operations".to_string());
        }

        // Serialization
        if content.contains("serde") || content.contains("jackson") || content.contains("gson") {
            insights.push("Data serialization/deserialization".to_string());
        }

        // Messaging
        if content.contains("kafka") || content.contains("rabbitmq") || content.contains("nats") {
            insights.push("Message queue integration".to_string());
        }

        // Testing
        if content.contains("testcontainers") || content.contains("wiremock") {
            insights.push("Integration testing with external services".to_string());
        }

        insights
    }

    fn analyze_docker_compose(&self, content: &str) -> Vec<String> {
        let mut insights = Vec::new();

        if content.contains("postgres") || content.contains("mysql") {
            insights.push("PostgreSQL/MySQL database service".to_string());
        }
        if content.contains("redis") {
            insights.push("Redis caching service".to_string());
        }
        if content.contains("kafka") || content.contains("zookeeper") {
            insights.push("Kafka message streaming".to_string());
        }
        if content.contains("nginx") || content.contains("traefik") {
            insights.push("Reverse proxy/load balancer".to_string());
        }

        insights
    }

    fn analyze_dockerfile(&self, content: &str) -> Vec<String> {
        let mut insights = Vec::new();

        if content.contains("FROM rust") || content.contains("cargo build") {
            insights.push("Rust application deployment".to_string());
        }
        if content.contains("EXPOSE") {
            insights.push("Network service with exposed ports".to_string());
        }
        if content.contains("multi-stage") || content.lines().filter(|l| l.starts_with("FROM")).count() > 1 {
            insights.push("Multi-stage build for optimization".to_string());
        }

        insights
    }

    fn analyze_project_config(&self, content: &str) -> Vec<String> {
        let mut insights = Vec::new();

        if content.contains("bin") || content.contains("[[bin]]") {
            insights.push("Executable application".to_string());
        }
        if content.contains("workspace") {
            insights.push("Multi-package workspace".to_string());
        }

        insights
    }

    // System context analysis

    async fn analyze_package_relationships<P: AsRef<Path>>(&self, project_root: P, package_names: &[String]) -> Result<Vec<RelatedPackage>> {
        let mut relationships = Vec::new();

        // For now, simple heuristic-based relationship detection
        // In a full implementation, this would analyze import graphs, etc.

        for package_name in package_names {
            if package_name.contains("api") || package_name.contains("handler") {
                relationships.push(RelatedPackage {
                    name: package_name.clone(),
                    relationship: "provides HTTP interface".to_string(),
                    interaction_summary: "Handles external HTTP requests and responses".to_string(),
                });
            } else if package_name.contains("service") {
                relationships.push(RelatedPackage {
                    name: package_name.clone(),
                    relationship: "business logic layer".to_string(),
                    interaction_summary: "Implements core business operations".to_string(),
                });
            } else if package_name.contains("repository") || package_name.contains("dao") {
                relationships.push(RelatedPackage {
                    name: package_name.clone(),
                    relationship: "data access layer".to_string(),
                    interaction_summary: "Manages data persistence and retrieval".to_string(),
                });
            }
        }

        Ok(relationships)
    }

    async fn detect_common_patterns<P: AsRef<Path>>(&self, project_root: P) -> Result<Vec<PatternUsage>> {
        let mut patterns = Vec::new();

        // Scan for common architectural patterns
        self.scan_for_patterns(project_root.as_ref(), &mut patterns).await?;

        Ok(patterns)
    }

    async fn scan_for_patterns<P: AsRef<Path>>(&self, dir: P, patterns: &mut Vec<PatternUsage>) -> Result<()> {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                        if matches!(dir_name, "target" | "node_modules" | ".git") {
                            continue;
                        }
                    }
                    Box::pin(self.scan_for_patterns(&path, patterns)).await?;
                } else if self.is_source_code_file(&path) {
                    self.detect_patterns_in_file(&path, patterns)?;
                }
            }
        }
        Ok(())
    }

    fn detect_patterns_in_file(&self, file_path: &Path, patterns: &mut Vec<PatternUsage>) -> Result<()> {
        let content = std::fs::read_to_string(file_path)?;

        // Repository pattern
        if content.contains("Repository") && (content.contains("save") || content.contains("find")) {
            patterns.push(PatternUsage {
                pattern_name: "Repository Pattern".to_string(),
                usage_context: "Data access abstraction".to_string(),
                benefits: vec![
                    "Separates data access logic".to_string(),
                    "Enables testing with mocks".to_string(),
                    "Provides consistent data access interface".to_string(),
                ],
            });
        }

        // Factory pattern
        if content.contains("Factory") || (content.contains("create") && content.contains("new")) {
            patterns.push(PatternUsage {
                pattern_name: "Factory Pattern".to_string(),
                usage_context: "Object creation abstraction".to_string(),
                benefits: vec![
                    "Encapsulates object creation logic".to_string(),
                    "Enables different creation strategies".to_string(),
                ],
            });
        }

        // Observer/Event pattern
        if content.contains("Event") && (content.contains("listener") || content.contains("handler")) {
            patterns.push(PatternUsage {
                pattern_name: "Observer/Event Pattern".to_string(),
                usage_context: "Loose coupling between components".to_string(),
                benefits: vec![
                    "Decouples event producers from consumers".to_string(),
                    "Enables extensible event handling".to_string(),
                ],
            });
        }

        Ok(())
    }

    async fn extract_architectural_themes<P: AsRef<Path>>(&self, project_root: P) -> Result<Vec<ArchitecturalTheme>> {
        let mut themes = Vec::new();

        // Look for evidence of major architectural approaches
        self.scan_for_architectural_themes(project_root.as_ref(), &mut themes).await?;

        Ok(themes)
    }

    async fn scan_for_architectural_themes<P: AsRef<Path>>(&self, dir: P, themes: &mut Vec<ArchitecturalTheme>) -> Result<()> {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && self.is_source_code_file(&path) {
                    self.detect_themes_in_file(&path, themes)?;
                }
            }
        }
        Ok(())
    }

    fn detect_themes_in_file(&self, file_path: &Path, themes: &mut Vec<ArchitecturalTheme>) -> Result<()> {
        let content = std::fs::read_to_string(file_path)?;

        // Event Sourcing
        if content.contains("EventStore") || content.contains("Event") && content.contains("apply") {
            themes.push(ArchitecturalTheme {
                theme: "Event Sourcing".to_string(),
                manifestation: "Events stored as source of truth".to_string(),
                rationale: "Provides complete audit trail and temporal queries".to_string(),
            });
        }

        // CQRS
        if (content.contains("Command") && content.contains("Query")) ||
            (content.contains("CommandHandler") && content.contains("QueryHandler")) {
            themes.push(ArchitecturalTheme {
                theme: "CQRS".to_string(),
                manifestation: "Separate command and query models".to_string(),
                rationale: "Optimizes read and write operations independently".to_string(),
            });
        }

        // Hexagonal Architecture
        if content.contains("Port") && content.contains("Adapter") {
            themes.push(ArchitecturalTheme {
                theme: "Hexagonal Architecture".to_string(),
                manifestation: "Ports and adapters for external interfaces".to_string(),
                rationale: "Isolates business logic from external concerns".to_string(),
            });
        }

        Ok(())
    }
}

impl Default for ContextScanner {
    fn default() -> Self {
        Self::new().expect("Failed to create ContextScanner")
    }
}