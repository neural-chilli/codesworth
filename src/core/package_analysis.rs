// src/core/package_analysis.rs
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::collections::HashMap;

use super::{ParsedFile, ParsedModule};

/// Package-level analysis result that groups related files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageAnalysis {
    /// Package identifier (e.g., "analytics", "user_service")
    pub package_name: String,

    /// Root path of the package
    pub package_path: PathBuf,

    /// All files in this package
    pub files: Vec<ParsedFile>,

    /// Consolidated public API surface
    pub public_api: ApiSurface,

    /// Dependency analysis
    pub dependencies: DependencyAnalysis,

    /// Complexity and architectural significance indicators
    pub complexity_indicators: ComplexityMetrics,

    /// Package-level documentation (from README, module docs, etc.)
    pub package_docs: Option<String>,

    /// Content hash representing the entire package state
    pub package_hash: String,
}

/// Represents the public API surface of a package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiSurface {
    /// Public functions exported by this package
    pub functions: Vec<ApiFunction>,

    /// Public types (structs, enums, interfaces, classes)
    pub types: Vec<ApiType>,

    /// Public constants and static values
    pub constants: Vec<ApiConstant>,

    /// Entry points and main interfaces
    pub entry_points: Vec<EntryPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiFunction {
    pub name: String,
    pub signature: String,
    pub docs: Option<String>,
    pub file_path: PathBuf,
    pub line_number: usize,
    pub is_async: bool,
    pub complexity_score: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiType {
    pub name: String,
    pub type_kind: String, // struct, enum, interface, class, trait
    pub docs: Option<String>,
    pub file_path: PathBuf,
    pub line_number: usize,
    pub methods: Vec<ApiFunction>,
    pub is_public: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConstant {
    pub name: String,
    pub type_name: String,
    pub value: Option<String>,
    pub docs: Option<String>,
    pub file_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryPoint {
    pub name: String,
    pub entry_type: EntryPointType,
    pub description: Option<String>,
    pub file_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntryPointType {
    Main,           // main() function
    HttpHandler,    // REST endpoint or handler
    EventHandler,   // Event/message handler
    PublicApi,      // Main public interface
    Configuration,  // Config or setup entry point
}

/// Analysis of package dependencies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyAnalysis {
    /// External dependencies (crates, npm packages, etc.)
    pub external_deps: Vec<ExternalDependency>,

    /// Internal dependencies (other packages in the project)
    pub internal_deps: Vec<InternalDependency>,

    /// Unusual or noteworthy imports that suggest architectural patterns
    pub unusual_imports: Vec<UnusualImport>,

    /// Circular dependency warnings
    pub circular_deps: Vec<CircularDependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalDependency {
    pub name: String,
    pub version: Option<String>,
    pub usage_context: String,  // web framework, database, etc.
    pub criticality: DependencyCriticality,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InternalDependency {
    pub package_name: String,
    pub relationship_type: RelationshipType,
    pub coupling_strength: CouplingStrength,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnusualImport {
    pub import_path: String,
    pub reason: String,      // Why it's notable
    pub file_path: PathBuf,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircularDependency {
    pub packages: Vec<String>,
    pub severity: CircularDependencySeverity,
    pub description: String,
}

/// Metrics indicating package complexity and architectural significance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityMetrics {
    /// Lines of code in the package
    pub loc: usize,

    /// Number of public interfaces
    pub public_interface_count: usize,

    /// Cyclomatic complexity estimate
    pub cyclomatic_complexity: u32,

    /// Number of external dependencies
    pub external_dependency_count: usize,

    /// Architectural significance score (0.0 to 1.0)
    pub architectural_significance: f32,

    /// Gotcha indicators (potential issues)
    pub gotchas: Vec<GotchaIndicator>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GotchaIndicator {
    pub category: GotchaCategory,
    pub description: String,
    pub severity: GotchaSeverity,
    pub file_path: Option<PathBuf>,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GotchaCategory {
    Concurrency,      // Potential deadlocks, race conditions
    Performance,      // Performance assumptions, bottlenecks
    Security,         // Security considerations
    Configuration,    // Config dependencies, environment assumptions
    ErrorHandling,    // Error handling patterns, failure modes
    State,           // State management complexity
    Integration,     // Integration points, API contracts
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GotchaSeverity {
    Info,      // Good to know
    Warning,   // Could cause issues
    Critical,  // Likely to cause problems
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DependencyCriticality {
    Core,       // Essential to functionality
    Important,  // Significant feature dependency
    Optional,   // Nice to have or utility
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationshipType {
    Uses,        // This package uses the other
    Implements,  // This package implements interface from other
    Extends,     // This package extends functionality of other
    Configures,  // This package configures the other
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CouplingStrength {
    Tight,    // Heavily coupled, changes propagate
    Medium,   // Some coupling, occasional impact
    Loose,    // Minimal coupling, independent changes
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CircularDependencySeverity {
    Error,    // Must be fixed
    Warning,  // Should be addressed
    Info,     // Architectural note
}

impl PackageAnalysis {
    /// Calculate the overall complexity score for prioritization
    pub fn complexity_score(&self) -> f32 {
        let metrics = &self.complexity_indicators;

        // Weighted scoring based on various factors
        let loc_score = (metrics.loc as f32 / 1000.0).min(1.0);
        let interface_score = (metrics.public_interface_count as f32 / 20.0).min(1.0);
        let dependency_score = (metrics.external_dependency_count as f32 / 10.0).min(1.0);
        let complexity_score = (metrics.cyclomatic_complexity as f32 / 100.0).min(1.0);

        // Weight the scores
        (loc_score * 0.2
            + interface_score * 0.3
            + dependency_score * 0.2
            + complexity_score * 0.2
            + metrics.architectural_significance * 0.1)
    }

    /// Check if this package needs immediate documentation attention
    pub fn needs_priority_documentation(&self) -> bool {
        self.complexity_score() > 0.7 ||
            self.complexity_indicators.gotchas.iter()
                .any(|g| matches!(g.severity, GotchaSeverity::Critical))
    }

    /// Get a human-readable summary of what this package does
    pub fn generate_summary(&self) -> String {
        // Based on entry points and API surface, infer purpose
        let entry_types: Vec<_> = self.public_api.entry_points
            .iter()
            .map(|ep| &ep.entry_type)
            .collect();

        if entry_types.iter().any(|t| matches!(t, EntryPointType::HttpHandler)) {
            format!("Web service package with {} HTTP handlers",
                    entry_types.iter().filter(|t| matches!(t, EntryPointType::HttpHandler)).count())
        } else if entry_types.iter().any(|t| matches!(t, EntryPointType::EventHandler)) {
            format!("Event processing package with {} handlers",
                    entry_types.iter().filter(|t| matches!(t, EntryPointType::EventHandler)).count())
        } else if !self.public_api.types.is_empty() {
            format!("Library package exposing {} public types", self.public_api.types.len())
        } else if !self.public_api.functions.is_empty() {
            format!("Utility package with {} public functions", self.public_api.functions.len())
        } else {
            "Internal package".to_string()
        }
    }
}