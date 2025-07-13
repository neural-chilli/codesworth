// src/core/call_graph/entry_point_detector.rs
use std::collections::HashSet;
use serde::{Serialize, Deserialize};

use crate::error::Result;
use super::{CallGraph, MethodSignature, CallNode};

/// Universal entry point detector that works across languages/frameworks
pub struct EntryPointDetector {
    /// Known entry point patterns by name
    known_entry_patterns: HashSet<String>,
    /// Known entry point annotations/decorators
    known_annotations: HashSet<String>,
}

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
    /// HTTP/Web endpoint
    HttpEndpoint,
    /// Event/message handler
    EventHandler,
    /// Scheduled/timer task
    ScheduledTask,
    /// CLI command handler
    CliCommand,
    /// Test entry point
    Test,
    /// Library public API
    PublicApi,
    /// Unknown/inferred from call graph
    Unknown,
}

impl EntryPointDetector {
    pub fn new() -> Self {
        let mut known_entry_patterns = HashSet::new();
        // Universal entry point names
        known_entry_patterns.insert("main".to_string());
        known_entry_patterns.insert("run".to_string());
        known_entry_patterns.insert("start".to_string());
        known_entry_patterns.insert("execute".to_string());
        known_entry_patterns.insert("handle".to_string());
        known_entry_patterns.insert("process".to_string());
        known_entry_patterns.insert("onMessage".to_string());
        known_entry_patterns.insert("on_message".to_string());

        let mut known_annotations = HashSet::new();
        // Framework annotations that indicate entry points
        known_annotations.insert("@RequestMapping".to_string());
        known_annotations.insert("@GetMapping".to_string());
        known_annotations.insert("@PostMapping".to_string());
        known_annotations.insert("@PutMapping".to_string());
        known_annotations.insert("@DeleteMapping".to_string());
        known_annotations.insert("@MessageMapping".to_string());
        known_annotations.insert("@EventHandler".to_string());
        known_annotations.insert("@Scheduled".to_string());
        known_annotations.insert("@Test".to_string());
        known_annotations.insert("@Command".to_string());
        // Python decorators
        known_annotations.insert("@app.route".to_string());
        known_annotations.insert("@router.get".to_string());
        known_annotations.insert("@router.post".to_string());
        known_annotations.insert("@celery.task".to_string());
        // JavaScript/Express patterns
        known_annotations.insert("app.get".to_string());
        known_annotations.insert("app.post".to_string());
        known_annotations.insert("router.get".to_string());
        known_annotations.insert("router.post".to_string());

        Self {
            known_entry_patterns,
            known_annotations,
        }
    }

    /// Detect all entry points in the call graph
    pub fn detect_entry_points(&self, call_graph: &CallGraph) -> Result<Vec<EntryPoint>> {
        let mut entry_points = Vec::new();

        // Primary strategy: Call graph analysis (in-degree=0, out-degree>0)
        let candidates = call_graph.get_entry_point_candidates();

        for candidate in candidates {
            let entry_point = self.analyze_entry_point_candidate(call_graph, candidate)?;
            entry_points.push(entry_point);
        }

        // Secondary strategy: Pattern and annotation detection
        // (for methods that might be entry points but have callers due to frameworks)
        for (signature, node) in &call_graph.nodes {
            if let Some(entry_point) = self.check_patterns_and_annotations(signature, node)? {
                // Only add if not already found
                if !entry_points.iter().any(|ep| ep.signature == *signature) {
                    entry_points.push(entry_point);
                }
            }
        }

        // Sort by confidence for better user experience
        entry_points.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));

        Ok(entry_points)
    }

    /// Analyze a call graph candidate to determine if it's a valid entry point
    fn analyze_entry_point_candidate(
        &self,
        call_graph: &CallGraph,
        candidate: &MethodSignature,
    ) -> Result<EntryPoint> {
        let node = call_graph.nodes.get(candidate).unwrap();

        let mut confidence: f32 = 0.5; // Base confidence for call graph candidates
        let mut reasoning = vec!["No incoming calls, has outgoing calls".to_string()];

        // Boost confidence based on naming patterns
        let entry_type = if self.is_main_function(candidate) {
            confidence += 0.4;
            reasoning.push("Named 'main' - application entry point".to_string());
            EntryPointType::Main
        } else if self.is_handler_function(candidate) {
            confidence += 0.3;
            reasoning.push("Named like handler - likely event/message handler".to_string());
            EntryPointType::EventHandler
        } else if self.is_test_function(candidate) {
            confidence += 0.2;
            reasoning.push("Named like test - test entry point".to_string());
            EntryPointType::Test
        } else if node.visibility == "public" || node.visibility == "pub" {
            confidence += 0.2;
            reasoning.push("Public visibility - potential library API".to_string());
            EntryPointType::PublicApi
        } else {
            EntryPointType::Unknown
        };

        // Boost confidence based on complexity (entry points often do work)
        if node.complexity_score > 10 {
            confidence += 0.1;
            reasoning.push("Non-trivial complexity".to_string());
        }

        // Check out-degree (entry points often call multiple methods)
        let out_degree = call_graph.out_degree(candidate);
        if out_degree > 3 {
            confidence += 0.1;
            reasoning.push(format!("Calls {} methods", out_degree));
        }

        Ok(EntryPoint {
            signature: candidate.clone(),
            entry_type,
            confidence: confidence.min(1.0),
            reasoning: reasoning.join("; "),
        })
    }

    /// Check for pattern and annotation-based entry points
    fn check_patterns_and_annotations(
        &self,
        signature: &MethodSignature,
        node: &CallNode,
    ) -> Result<Option<EntryPoint>> {
        // Read the source around this method to look for annotations
        if let Ok(source) = std::fs::read_to_string(&signature.file_path) {
            let lines: Vec<&str> = source.lines().collect();

            // Look for annotations in the lines before the method
            let method_line = node.line_range.0.saturating_sub(1); // Convert to 0-based
            let search_start = method_line.saturating_sub(5); // Look 5 lines before

            for i in search_start..method_line {
                if i < lines.len() {
                    let line = lines[i].trim();

                    for annotation in &self.known_annotations {
                        if line.contains(annotation) {
                            let (entry_type, confidence) = self.classify_annotation(annotation);
                            return Ok(Some(EntryPoint {
                                signature: signature.clone(),
                                entry_type,
                                confidence,
                                reasoning: format!("Has annotation: {}", annotation),
                            }));
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Classify entry point type based on annotation
    fn classify_annotation(&self, annotation: &str) -> (EntryPointType, f32) {
        match annotation {
            s if s.contains("Mapping") || s.contains("route") => (EntryPointType::HttpEndpoint, 0.9),
            s if s.contains("Message") || s.contains("Event") => (EntryPointType::EventHandler, 0.8),
            s if s.contains("Scheduled") || s.contains("task") => (EntryPointType::ScheduledTask, 0.8),
            s if s.contains("Test") => (EntryPointType::Test, 0.7),
            s if s.contains("Command") => (EntryPointType::CliCommand, 0.8),
            _ => (EntryPointType::Unknown, 0.6),
        }
    }

    /// Check if this looks like a main function
    fn is_main_function(&self, signature: &MethodSignature) -> bool {
        signature.method_name == "main"
    }

    /// Check if this looks like a handler function
    fn is_handler_function(&self, signature: &MethodSignature) -> bool {
        let name = signature.method_name.to_lowercase();
        name.contains("handle") ||
            name.contains("process") ||
            name.contains("on_") ||
            name.starts_with("on") ||
            name.ends_with("handler")
    }

    /// Check if this looks like a test function
    fn is_test_function(&self, signature: &MethodSignature) -> bool {
        let name = signature.method_name.to_lowercase();
        name.starts_with("test_") ||
            name.starts_with("test") ||
            name.ends_with("_test") ||
            name.contains("should_")
    }
}

impl Default for EntryPointDetector {
    fn default() -> Self {
        Self::new()
    }
}