// src/core/call_graph/mod.rs
//! Call-chain analysis system for Codesworth
//!
//! This module implements the core call-chain analysis approach that traces
//! execution paths through codebases to generate natural, workflow-oriented
//! documentation.

mod call_graph;
mod entry_point_detector;
mod call_chain_tracer;
mod call_chain_grouper;
mod call_chain_analyzer;
mod call_chain_engine;

pub use call_graph::{CallGraph, CallNode, CallEdge, MethodSignature, CallType, CallGraphStats};
pub use entry_point_detector::{EntryPointDetector, EntryPoint, EntryPointType};
pub use call_chain_tracer::{CallChainTracer, CallChain, CallStep};
pub use call_chain_grouper::{CallChainGrouper, CallChainGroup, GroupingStats};
pub use call_chain_analyzer::{
    CallChainAnalyzer, GroupAnalysis, VisitedSet, ComponentInteraction,
    DomainInsight, Gotcha, GotchaSeverity, SystemSynthesis
};
pub use call_chain_engine::{CallChainEngine, CallChainAnalysisResult, AnalysisStatistics};

// Re-export needed types from other modules for internal use
pub use super::llm::{LlmDocumenter, EnhancementRequest, EnhancementType, DocumentationContext, ProjectInfo};
pub use super::{ParsedFile, CodeParser};