// src/core/call_graph/entry_point_detector.rs - Simple universal approach
use std::collections::HashSet;
use serde::{Serialize, Deserialize};

use crate::error::Result;
use super::{CallGraph, MethodSignature, CallNode};

/// Universal entry point detector - simple and framework-agnostic
pub struct EntryPointDetector;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryPoint {
    pub signature: MethodSignature,
    pub entry_type: EntryPointType,
    pub confidence: f32,
    pub reasoning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntryPointType {
    /// Main application entry point
    Main,
    /// External interface (no callers, has callees)
    ExternalInterface,
    /// Test entry point
    Test,
    /// Public API method
    PublicApi,
}

impl EntryPointDetector {
    pub fn new() -> Self {
        Self
    }

    /// Detect all entry points in the call graph
    /// Simple rule: methods with no callers that call other methods
    pub fn detect_entry_points(&self, call_graph: &CallGraph) -> Result<Vec<EntryPoint>> {
        let mut entry_points = Vec::new();

        // Primary strategy: Call graph analysis (in-degree=0, out-degree>0)
        let candidates = call_graph.get_entry_point_candidates();

        println!("Found {} entry point candidates", candidates.len());

        for candidate in candidates {
            let entry_point = self.analyze_entry_point(call_graph, candidate)?;
            entry_points.push(entry_point);
        }

        // Also check for main methods specifically
        for (signature, node) in &call_graph.nodes {
            if signature.method_name == "main" && !entry_points.iter().any(|ep| ep.signature == *signature) {
                let entry_point = EntryPoint {
                    signature: signature.clone(),
                    entry_type: EntryPointType::Main,
                    confidence: 0.95,
                    reasoning: "Main method".to_string(),
                };
                entry_points.push(entry_point);
            }
        }

        // Sort by confidence
        entry_points.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));

        Ok(entry_points)
    }

    /// Analyze a call graph candidate
    fn analyze_entry_point(
        &self,
        call_graph: &CallGraph,
        candidate: &MethodSignature,
    ) -> Result<EntryPoint> {
        let node = call_graph.nodes.get(candidate).unwrap();

        let mut confidence: f32 = 0.7; // Base confidence for call graph candidates
        let mut reasoning_parts = vec!["No incoming calls, has outgoing calls".to_string()];

        // Classify entry point type based on method characteristics
        let entry_type = if candidate.method_name == "main" {
            confidence = 0.95;
            reasoning_parts.push("Main application entry point".to_string());
            EntryPointType::Main
        } else if candidate.method_name.contains("test") || candidate.method_name.starts_with("test") {
            confidence = 0.8;
            reasoning_parts.push("Test method pattern".to_string());
            EntryPointType::Test
        } else if node.visibility == "public" {
            confidence = 0.8;
            reasoning_parts.push("Public method - likely external interface".to_string());
            EntryPointType::ExternalInterface
        } else {
            confidence = 0.6;
            reasoning_parts.push("Package-visible method".to_string());
            EntryPointType::PublicApi
        };

        // Boost confidence based on out-degree
        let out_degree = call_graph.out_degree(candidate);
        if out_degree > 3 {
            confidence += 0.1;
            reasoning_parts.push(format!("Calls {} other methods", out_degree));
        }

        // Boost confidence based on complexity
        if node.complexity_score > 10 {
            confidence += 0.05;
            reasoning_parts.push("Non-trivial complexity".to_string());
        }

        Ok(EntryPoint {
            signature: candidate.clone(),
            entry_type,
            confidence: confidence.min(1.0),
            reasoning: reasoning_parts.join(", "),
        })
    }
}

impl Default for EntryPointDetector {
    fn default() -> Self {
        Self::new()
    }
}