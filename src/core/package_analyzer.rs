// src/core/package_analyzer.rs
use std::path::{Path, PathBuf};
use std::collections::{HashMap, HashSet};
use sha2::{Sha256, Digest};

use crate::error::{CodesworthError, Result};
use crate::config::ParsingConfig;
use super::{
    ParsedFile, ParsedModule, CodeParser,
    package_analysis::*
};

/// Analyzes and groups files into logical packages for documentation
pub struct PackageAnalyzer {
    config: ParsingConfig,
}

impl PackageAnalyzer {
    pub fn new(config: &ParsingConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    /// Analyze a directory and group files into packages
    pub async fn analyze_directory<P: AsRef<Path>>(&self, dir: P, parser: &mut CodeParser) -> Result<Vec<PackageAnalysis>> {
        // First, parse all files
        let parsed_files = parser.parse_directory(&dir).await?;

        // Group files into logical packages
        let package_groups = self.group_files_into_packages(&parsed_files)?;

        // Analyze each package
        let mut package_analyses = Vec::new();
        for (package_name, files) in package_groups {
            let analysis = self.analyze_package(&package_name, files).await?;
            package_analyses.push(analysis);
        }

        // Sort by complexity/priority for better user experience
        package_analyses.sort_by(|a, b| {
            b.complexity_score().partial_cmp(&a.complexity_score()).unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(package_analyses)
    }

    /// Group parsed files into logical packages based on directory structure and content
    fn group_files_into_packages(&self, files: &[ParsedFile]) -> Result<HashMap<String, Vec<ParsedFile>>> {
        let mut packages: HashMap<String, Vec<ParsedFile>> = HashMap::new();

        for file in files {
            let package_name = self.determine_package_name(&file.path)?;
            packages.entry(package_name)
                .or_insert_with(Vec::new)
                .push(file.clone());
        }

        Ok(packages)
    }

    /// Determine which package a file belongs to based on path and content
    fn determine_package_name(&self, file_path: &Path) -> Result<String> {
        let path_str = file_path.to_string_lossy();

        // Language-specific package detection
        if path_str.contains("/src/") {
            // Rust-style: src/analytics/mod.rs -> "analytics"
            if let Some(src_index) = path_str.find("/src/") {
                let after_src = &path_str[src_index + 5..];
                if let Some(first_slash) = after_src.find('/') {
                    let package = &after_src[..first_slash];
                    if package != "bin" && package != "test" && package != "tests" {
                        return Ok(package.to_string());
                    }
                }
            }
        }

        // Java-style: src/main/java/com/company/service -> "service"
        if path_str.contains("/java/") {
            if let Some(java_index) = path_str.find("/java/") {
                let after_java = &path_str[java_index + 6..];
                let parts: Vec<&str> = after_java.split('/').collect();
                if parts.len() > 3 {
                    // Take the last meaningful part before filename
                    return Ok(parts[parts.len() - 2].to_string());
                }
            }
        }

        // Python-style: package detection from __init__.py or directory structure
        if path_str.ends_with("__init__.py") {
            if let Some(parent) = file_path.parent() {
                if let Some(package_name) = parent.file_name() {
                    return Ok(package_name.to_string_lossy().to_string());
                }
            }
        }

        // JavaScript/TypeScript: src/components/user -> "user"
        if path_str.contains("/src/") && (path_str.ends_with(".js") || path_str.ends_with(".ts")) {
            if let Some(src_index) = path_str.find("/src/") {
                let after_src = &path_str[src_index + 5..];
                let parts: Vec<&str> = after_src.split('/').collect();
                if parts.len() > 1 {
                    return Ok(parts[0].to_string());
                }
            }
        }

        // Fallback: use parent directory name
        if let Some(parent) = file_path.parent() {
            if let Some(dir_name) = parent.file_name() {
                Ok(dir_name.to_string_lossy().to_string())
            } else {
                Ok("root".to_string())
            }
        } else {
            Ok("root".to_string())
        }
    }

    /// Perform comprehensive analysis of a package
    async fn analyze_package(&self, package_name: &str, files: Vec<ParsedFile>) -> Result<PackageAnalysis> {
        // Calculate package path (common root of all files)
        let package_path = self.calculate_package_path(&files);

        // Extract public API surface
        let public_api = self.extract_public_api(&files)?;

        // Analyze dependencies
        let dependencies = self.analyze_dependencies(&files)?;

        // Calculate complexity metrics
        let complexity_indicators = self.calculate_complexity_metrics(&files, &dependencies)?;

        // Extract package-level documentation
        let package_docs = self.extract_package_docs(&files);

        // Calculate package hash
        let package_hash = self.calculate_package_hash(&files);

        Ok(PackageAnalysis {
            package_name: package_name.to_string(),
            package_path,
            files,
            public_api,
            dependencies,
            complexity_indicators,
            package_docs,
            package_hash,
        })
    }

    fn calculate_package_path(&self, files: &[ParsedFile]) -> PathBuf {
        if files.is_empty() {
            return PathBuf::from(".");
        }

        // Find the common parent directory
        let mut common_path = files[0].path.parent().unwrap_or(Path::new(".")).to_path_buf();

        for file in files.iter().skip(1) {
            if let Some(parent) = file.path.parent() {
                common_path = self.find_common_path(&common_path, parent);
            }
        }

        common_path
    }

    fn find_common_path(&self, path1: &Path, path2: &Path) -> PathBuf {
        let components1: Vec<_> = path1.components().collect();
        let components2: Vec<_> = path2.components().collect();

        let mut common = PathBuf::new();
        for (c1, c2) in components1.iter().zip(components2.iter()) {
            if c1 == c2 {
                common.push(c1);
            } else {
                break;
            }
        }

        common
    }

    fn extract_public_api(&self, files: &[ParsedFile]) -> Result<ApiSurface> {
        let mut functions = Vec::new();
        let mut types = Vec::new();
        let mut constants = Vec::new();
        let mut entry_points = Vec::new();

        for file in files {
            for module in &file.modules {
                if module.visibility == "public" || module.visibility == "pub" {
                    match module.item_type.as_str() {
                        "function" | "method" => {
                            functions.push(ApiFunction {
                                name: module.name.clone(),
                                signature: module.signature.clone().unwrap_or_default(),
                                docs: module.docs.clone(),
                                file_path: file.path.clone(),
                                line_number: module.line_range.0,
                                is_async: module.signature.as_ref()
                                    .map_or(false, |s| s.contains("async")),
                                complexity_score: self.estimate_function_complexity(module),
                            });

                            // Detect entry points
                            if self.is_entry_point(module) {
                                entry_points.push(EntryPoint {
                                    name: module.name.clone(),
                                    entry_type: self.classify_entry_point(module),
                                    description: module.docs.clone(),
                                    file_path: file.path.clone(),
                                });
                            }
                        }
                        "struct" | "class" | "interface" | "enum" | "trait" => {
                            let methods = module.children.iter()
                                .filter(|child| child.visibility == "public" || child.visibility == "pub")
                                .map(|child| ApiFunction {
                                    name: child.name.clone(),
                                    signature: child.signature.clone().unwrap_or_default(),
                                    docs: child.docs.clone(),
                                    file_path: file.path.clone(),
                                    line_number: child.line_range.0,
                                    is_async: child.signature.as_ref()
                                        .map_or(false, |s| s.contains("async")),
                                    complexity_score: self.estimate_function_complexity(child),
                                })
                                .collect();

                            types.push(ApiType {
                                name: module.name.clone(),
                                type_kind: module.item_type.clone(),
                                docs: module.docs.clone(),
                                file_path: file.path.clone(),
                                line_number: module.line_range.0,
                                methods,
                                is_public: true,
                            });
                        }
                        "const" | "static" => {
                            constants.push(ApiConstant {
                                name: module.name.clone(),
                                type_name: "unknown".to_string(), // TODO: extract from signature
                                value: None, // TODO: extract if available
                                docs: module.docs.clone(),
                                file_path: file.path.clone(),
                            });
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(ApiSurface {
            functions,
            types,
            constants,
            entry_points,
        })
    }

    fn analyze_dependencies(&self, files: &[ParsedFile]) -> Result<DependencyAnalysis> {
        let mut external_deps = HashMap::new();
        let mut internal_deps = HashSet::new();
        let mut unusual_imports = Vec::new();

        for file in files {
            // Extract imports/uses from source content
            let imports = self.extract_imports(&file.source_content, &file.language);

            for import in imports {
                if self.is_external_dependency(&import) {
                    let criticality = self.assess_dependency_criticality(&import);
                    let usage_context = self.classify_dependency_usage(&import);

                    external_deps.insert(import.clone(), ExternalDependency {
                        name: import.clone(),
                        version: None, // TODO: extract from build files
                        usage_context,
                        criticality,
                    });
                } else if self.is_internal_dependency(&import) {
                    internal_deps.insert(import.clone());
                }

                if self.is_unusual_import(&import) {
                    unusual_imports.push(UnusualImport {
                        import_path: import.clone(),
                        reason: self.explain_unusual_import(&import),
                        file_path: file.path.clone(),
                        suggestion: self.suggest_import_alternative(&import),
                    });
                }
            }
        }

        let external_deps: Vec<_> = external_deps.into_values().collect();
        let internal_deps: Vec<_> = internal_deps.into_iter()
            .map(|dep| InternalDependency {
                package_name: dep,
                relationship_type: RelationshipType::Uses,
                coupling_strength: CouplingStrength::Medium,
            })
            .collect();

        Ok(DependencyAnalysis {
            external_deps,
            internal_deps,
            unusual_imports,
            circular_deps: vec![], // TODO: implement circular dependency detection
        })
    }

    fn calculate_complexity_metrics(&self, files: &[ParsedFile], dependencies: &DependencyAnalysis) -> Result<ComplexityMetrics> {
        let loc: usize = files.iter().map(|f| f.source_content.lines().count()).sum();
        let public_interface_count = files.iter()
            .flat_map(|f| &f.modules)
            .filter(|m| m.visibility == "public" || m.visibility == "pub")
            .count();

        let cyclomatic_complexity = self.estimate_cyclomatic_complexity(files);
        let external_dependency_count = dependencies.external_deps.len();
        let architectural_significance = self.calculate_architectural_significance(files, dependencies);
        let gotchas = self.detect_gotchas(files, dependencies);

        Ok(ComplexityMetrics {
            loc,
            public_interface_count,
            cyclomatic_complexity,
            external_dependency_count,
            architectural_significance,
            gotchas,
        })
    }

    fn extract_package_docs(&self, files: &[ParsedFile]) -> Option<String> {
        // Look for module-level docs, README files, or comprehensive file docs
        for file in files {
            if file.path.file_name().and_then(|n| n.to_str()) == Some("mod.rs") ||
                file.path.file_name().and_then(|n| n.to_str()) == Some("__init__.py") {
                if let Some(docs) = &file.file_docs {
                    return Some(docs.clone());
                }
            }
        }

        // Fallback to the first file with substantial documentation
        files.iter()
            .filter_map(|f| f.file_docs.as_ref())
            .find(|docs| docs.len() > 100)
            .cloned()
    }

    fn calculate_package_hash(&self, files: &[ParsedFile]) -> String {
        let mut hasher = Sha256::new();

        // Sort files by path for consistent hashing
        let mut sorted_files = files.to_vec();
        sorted_files.sort_by(|a, b| a.path.cmp(&b.path));

        for file in sorted_files {
            hasher.update(file.content_hash.as_bytes());
        }

        format!("{:x}", hasher.finalize())
    }

    // Helper methods for analysis

    fn extract_imports(&self, source: &str, language: &str) -> Vec<String> {
        let mut imports = Vec::new();

        for line in source.lines() {
            let trimmed = line.trim();
            match language {
                "rust" => {
                    if trimmed.starts_with("use ") {
                        if let Some(import) = self.extract_rust_import(trimmed) {
                            imports.push(import);
                        }
                    }
                }
                "java" => {
                    if trimmed.starts_with("import ") {
                        if let Some(import) = self.extract_java_import(trimmed) {
                            imports.push(import);
                        }
                    }
                }
                "python" => {
                    if trimmed.starts_with("import ") || trimmed.starts_with("from ") {
                        if let Some(import) = self.extract_python_import(trimmed) {
                            imports.push(import);
                        }
                    }
                }
                "javascript" => {
                    if trimmed.contains("import ") || trimmed.contains("require(") {
                        if let Some(import) = self.extract_javascript_import(trimmed) {
                            imports.push(import);
                        }
                    }
                }
                _ => {}
            }
        }

        imports
    }

    fn extract_rust_import(&self, line: &str) -> Option<String> {
        // Extract crate name from "use crate_name::..." or "use ::crate_name::..."
        if let Some(use_part) = line.strip_prefix("use ") {
            let cleaned = use_part.trim_end_matches(';').trim();
            if let Some(first_part) = cleaned.split("::").next() {
                return Some(first_part.to_string());
            }
        }
        None
    }

    fn extract_java_import(&self, line: &str) -> Option<String> {
        if let Some(import_part) = line.strip_prefix("import ") {
            let cleaned = import_part.trim_end_matches(';').trim();
            // Extract package root (e.g., "com.company.service" -> "com")
            if let Some(first_part) = cleaned.split('.').next() {
                return Some(first_part.to_string());
            }
        }
        None
    }

    fn extract_python_import(&self, line: &str) -> Option<String> {
        if let Some(import_part) = line.strip_prefix("import ") {
            if let Some(first_part) = import_part.split('.').next() {
                return Some(first_part.trim().to_string());
            }
        } else if let Some(from_part) = line.strip_prefix("from ") {
            if let Some(module_part) = from_part.split(" import").next() {
                if let Some(first_part) = module_part.split('.').next() {
                    return Some(first_part.trim().to_string());
                }
            }
        }
        None
    }

    fn extract_javascript_import(&self, line: &str) -> Option<String> {
        // Handle both import statements and require calls
        if line.contains("from '") || line.contains("from \"") {
            // ES6 import
            let quote_char = if line.contains("from '") { '\'' } else { '"' };
            if let Some(from_pos) = line.find(&format!("from {}", quote_char)) {
                let after_from = &line[from_pos + 6..];
                if let Some(end_quote) = after_from.find(quote_char) {
                    let import_path = &after_from[..end_quote];
                    return Some(import_path.to_string());
                }
            }
        } else if line.contains("require(") {
            // CommonJS require
            if let Some(start) = line.find("require(") {
                let after_require = &line[start + 8..];
                if let Some(quote_start) = after_require.find(|c| c == '\'' || c == '"') {
                    let quote_char = after_require.chars().nth(quote_start).unwrap();
                    let after_quote = &after_require[quote_start + 1..];
                    if let Some(end_quote) = after_quote.find(quote_char) {
                        let import_path = &after_quote[..end_quote];
                        return Some(import_path.to_string());
                    }
                }
            }
        }
        None
    }

    fn is_external_dependency(&self, import: &str) -> bool {
        // Basic heuristics for external dependencies
        !import.starts_with('.') && !import.starts_with("/") && !import.starts_with("crate") &&
            !import.starts_with("super") && !import.starts_with("self")
    }

    fn is_internal_dependency(&self, import: &str) -> bool {
        import.starts_with("crate") || import.starts_with("super") || import.starts_with(".")
    }

    fn is_unusual_import(&self, import: &str) -> bool {
        // Flag potentially problematic imports
        import.contains("unsafe") ||
            import.contains("std::mem") ||
            import.contains("libc") ||
            import.contains("ffi") ||
            import.to_lowercase().contains("experimental")
    }

    fn explain_unusual_import(&self, import: &str) -> String {
        if import.contains("unsafe") {
            "Contains unsafe operations that require careful review".to_string()
        } else if import.contains("std::mem") {
            "Direct memory manipulation - ensure safety".to_string()
        } else if import.contains("libc") {
            "C library bindings - platform-specific considerations".to_string()
        } else if import.contains("ffi") {
            "Foreign function interface - ABI stability concerns".to_string()
        } else {
            "Unusual import that may indicate complex patterns".to_string()
        }
    }

    fn suggest_import_alternative(&self, _import: &str) -> Option<String> {
        // TODO: Implement suggestions based on common patterns
        None
    }

    fn assess_dependency_criticality(&self, _dependency: &str) -> DependencyCriticality {
        // TODO: Implement based on known critical dependencies
        DependencyCriticality::Important
    }

    fn classify_dependency_usage(&self, dependency: &str) -> String {
        if dependency.to_lowercase().contains("serde") {
            "Serialization".to_string()
        } else if dependency.to_lowercase().contains("tokio") || dependency.to_lowercase().contains("async") {
            "Async Runtime".to_string()
        } else if dependency.to_lowercase().contains("axum") || dependency.to_lowercase().contains("warp") {
            "Web Framework".to_string()
        } else if dependency.to_lowercase().contains("diesel") || dependency.to_lowercase().contains("sqlx") {
            "Database".to_string()
        } else {
            "Utility".to_string()
        }
    }

    fn is_entry_point(&self, module: &ParsedModule) -> bool {
        module.name == "main" ||
            module.name.contains("handler") ||
            module.name.contains("endpoint") ||
            module.name.contains("controller") ||
            module.name.starts_with("handle_")
    }

    fn classify_entry_point(&self, module: &ParsedModule) -> EntryPointType {
        if module.name == "main" {
            EntryPointType::Main
        } else if module.name.contains("handler") || module.name.contains("endpoint") {
            EntryPointType::HttpHandler
        } else if module.name.contains("event") {
            EntryPointType::EventHandler
        } else {
            EntryPointType::PublicApi
        }
    }

    fn estimate_function_complexity(&self, module: &ParsedModule) -> u32 {
        // Simple heuristic based on signature complexity
        let signature_complexity = module.signature.as_ref()
            .map(|s| s.matches(',').count() as u32 + 1)
            .unwrap_or(1);

        let line_complexity = (module.line_range.1 - module.line_range.0) as u32;

        signature_complexity + (line_complexity / 10)
    }

    fn estimate_cyclomatic_complexity(&self, files: &[ParsedFile]) -> u32 {
        // Simple estimate based on control flow keywords
        let mut complexity = 0;

        for file in files {
            let content = &file.source_content;
            complexity += content.matches("if ").count() as u32;
            complexity += content.matches("while ").count() as u32;
            complexity += content.matches("for ").count() as u32;
            complexity += content.matches("match ").count() as u32;
            complexity += content.matches("case ").count() as u32;
            complexity += content.matches("&&").count() as u32;
            complexity += content.matches("||").count() as u32;
        }

        complexity
    }

    fn calculate_architectural_significance(&self, files: &[ParsedFile], dependencies: &DependencyAnalysis) -> f32 {
        let mut significance: f32 = 0.0;

        // High significance indicators
        if dependencies.external_deps.len() > 5 {
            significance += 0.3;
        }

        if files.iter().any(|f| f.source_content.contains("pub trait") || f.source_content.contains("interface")) {
            significance += 0.2;
        }

        if files.iter().any(|f| f.source_content.contains("async") || f.source_content.contains("tokio")) {
            significance += 0.2;
        }

        let total_loc: usize = files.iter().map(|f| f.source_content.lines().count()).sum();
        if total_loc > 1000 {
            significance += 0.3;
        }

        significance.min(1.0)
    }

    fn detect_gotchas(&self, files: &[ParsedFile], _dependencies: &DependencyAnalysis) -> Vec<GotchaIndicator> {
        let mut gotchas = Vec::new();

        for file in files {
            let content = &file.source_content;

            // Concurrency gotchas
            if content.contains("Arc<Mutex<") && content.contains("await") {
                gotchas.push(GotchaIndicator {
                    category: GotchaCategory::Concurrency,
                    description: "Potential deadlock: Arc<Mutex<>> with async code".to_string(),
                    severity: GotchaSeverity::Warning,
                    file_path: Some(file.path.clone()),
                    suggestion: Some("Consider using async-aware synchronization primitives".to_string()),
                });
            }

            // Performance gotchas
            if content.contains("clone()") && content.lines().filter(|l| l.contains("clone()")).count() > 10 {
                gotchas.push(GotchaIndicator {
                    category: GotchaCategory::Performance,
                    description: "High clone() usage may impact performance".to_string(),
                    severity: GotchaSeverity::Info,
                    file_path: Some(file.path.clone()),
                    suggestion: Some("Review if references or borrowing could be used instead".to_string()),
                });
            }

            // Error handling gotchas
            if content.contains("unwrap()") {
                let unwrap_count = content.matches("unwrap()").count();
                if unwrap_count > 3 {
                    gotchas.push(GotchaIndicator {
                        category: GotchaCategory::ErrorHandling,
                        description: format!("Multiple unwrap() calls ({}) may cause panics", unwrap_count),
                        severity: GotchaSeverity::Warning,
                        file_path: Some(file.path.clone()),
                        suggestion: Some("Consider using proper error handling with ? operator or match".to_string()),
                    });
                }
            }
        }

        gotchas
    }
}