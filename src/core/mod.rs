mod engine;
mod parser;
mod generator;
mod differ;
mod protector;
mod validator;
mod llm;

// New package-level analysis
mod package_analysis;
mod package_analyzer;
mod batch_processor;
mod context_scanner;
mod hierarchical_analyzer;
mod system_overview_generator;

// Language-specific parsers
mod languages;

pub use engine::Engine;
pub use parser::{CodeParser, ParsedFile, ParsedModule};
pub use generator::{DocGenerator, GeneratedDoc};
pub use differ::{ContentDiffer, ContentDiff};
pub use protector::{EditProtector, ProtectedRegion};
pub use validator::{DocValidator, ValidationResult};
pub use llm::{
    LlmDocumenter, DocumentationContext, EnhancementRequest, EnhancementResponse,
    EnhancementType, ProjectInfo, ArchitectureDocs, ArchitectureDetector
};

// New package-level exports
pub use package_analysis::*;
pub use package_analyzer::PackageAnalyzer;
pub use batch_processor::{
    BatchProcessor, BatchDocumentationRequest, BatchDocumentationResponse,
    HumanContext, SystemContext, AnalysisFocus, FocusArea, DepthLevel, TargetAudience
};
pub use context_scanner::ContextScanner;
pub use hierarchical_analyzer::HierarchicalAnalyzer;
pub use system_overview_generator::SystemOverviewGenerator;