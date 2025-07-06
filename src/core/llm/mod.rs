//! LLM integration for enhancing generated documentation
//!
//! This module provides a trait-based architecture for integrating different
//! LLM providers to enhance documentation with intelligent descriptions,
//! usage examples, and architectural insights.

mod documenter;
mod providers;
mod architecture_detector;

pub use documenter::{
    LlmDocumenter, DocumentationContext, EnhancementRequest, EnhancementResponse,
    EnhancementType, ProjectInfo, DocumenterCapabilities, ArchitectureDocs
};
pub use providers::{RigProvider, CortexProvider, create_documenter};
pub use architecture_detector::ArchitectureDetector;

use crate::error::Result;
use crate::config::LlmConfig;