// src/core/call_graph/call_chain_tracer.rs
use std::collections::{HashSet, VecDeque};
use serde::{Serialize, Deserialize};

use crate::error::Result;
use super::{CallGraph, MethodSignature, EntryPoint, CallType};

/// Traces execution paths from entry points through the call graph
pub struct CallChainTracer {
    /// Maximum depth to trace (prevents infinite recursion)
    max_depth: usize,
    /// Whether to stop at cycles
    stop_at_cycles: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallChain {
    /// Entry point that starts this chain
    pub entry_point: EntryPoint,
    /// Sequence of method calls in execution order
    pub steps: Vec<CallStep>,
    /// All files involved in this call chain
    pub involved_files: HashSet<std::path::PathBuf>,
    /// Whether this chain contains cycles
    pub has_cycles: bool,
    /// Total complexity score for the chain
    pub complexity_score: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallStep {
    /// Method being called
    pub method: MethodSignature,
    /// Depth in the call chain (0 = entry point)
    pub depth: usize,
    /// Line where this call is made (0 for entry point)
    pub call_site_line: usize,
    /// Type of call (direct, async, etc.)
    pub call_type: CallType,
    /// Methods called from this step
    pub callees: Vec<MethodSignature>,
}

impl CallChainTracer {
    pub fn new(max_depth: usize) -> Self {
        Self {
            max_depth,
            stop_at_cycles: true,
        }
    }

    /// Trace call chains from all entry points
    pub fn trace_all_chains(&self, call_graph: &CallGraph, entry_points: &[EntryPoint]) -> Result<Vec<CallChain>> {
        let mut all_chains = Vec::new();

        for entry_point in entry_points {
            let chains = self.trace_from_entry_point(call_graph, entry_point)?;
            all_chains.extend(chains);
        }

        Ok(all_chains)
    }

    /// Trace call chains from a specific entry point
    pub fn trace_from_entry_point(&self, call_graph: &CallGraph, entry_point: &EntryPoint) -> Result<Vec<CallChain>> {
        // Simple implementation - create one basic chain per entry point
        let mut involved_files = HashSet::new();
        involved_files.insert(entry_point.signature.file_path.clone());

        let entry_step = CallStep {
            method: entry_point.signature.clone(),
            depth: 0,
            call_site_line: 0,
            call_type: CallType::Direct,
            callees: call_graph.get_callees(&entry_point.signature).into_iter().cloned().collect(),
        };

        let chain = CallChain {
            entry_point: entry_point.clone(),
            steps: vec![entry_step],
            involved_files,
            has_cycles: false,
            complexity_score: 10, // Placeholder
        };

        Ok(vec![chain])
    }
}

impl Default for CallChainTracer {
    fn default() -> Self {
        Self::new(6)
    }
}