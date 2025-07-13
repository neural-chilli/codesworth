// Fixed src/core/mod.rs
mod engine;
mod parser;
mod generator;
mod differ;
mod protector;
mod validator;
mod llm;

// Call graph analysis (replacing package-level analysis)
mod call_graph;

// Language-specific parsers
mod languages;

pub use parser::{CodeParser, ParsedFile, ParsedModule};
pub use generator::{DocGenerator, GeneratedDoc};
pub use differ::{ContentDiffer, ContentDiff};
pub use protector::{EditProtector, ProtectedRegion};
pub use validator::{DocValidator, ValidationResult};
pub use llm::{
    LlmDocumenter, DocumentationContext, EnhancementRequest, EnhancementResponse,
    EnhancementType, ProjectInfo, ArchitectureDocs, ArchitectureDetector
};

// New call graph exports
pub use call_graph::{
    CallGraph, CallNode, CallEdge, MethodSignature, CallType,
    EntryPointDetector, EntryPoint, EntryPointType,
    CallChainTracer, CallChain, CallStep,
    CallChainGrouper, CallChainGroup,
    CallChainAnalyzer, GroupAnalysis, VisitedSet,
    CallChainEngine, CallChainAnalysisResult
};

// Export the main engine
pub use engine::Engine;