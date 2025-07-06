//! Core engine for Codesworth documentation generation
//!
//! This module contains the main orchestration logic for:
//! - Code analysis and parsing
//! - Documentation generation
//! - Edit preservation
//! - Content synchronization
//! - LLM-powered enhancement

mod engine;
mod parser;
mod generator;
mod differ;
mod protector;
mod validator;
mod llm;

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