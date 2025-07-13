// src/core/call_graph/call_graph.rs - Fixed call graph building
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use serde::{Serialize, Deserialize};

use crate::error::Result;
use super::super::{ParsedFile, ParsedModule};

/// Unique identifier for a method/function in the codebase
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MethodSignature {
    /// File path
    pub file_path: PathBuf,
    /// Method/function name
    pub method_name: String,
    /// Class/struct name (if applicable)
    pub class_name: Option<String>,
    /// Namespace/module path
    pub namespace: Option<String>,
    /// Language-specific signature for disambiguation
    pub signature: String,
}

impl MethodSignature {
    pub fn new(
        file_path: PathBuf,
        method_name: String,
        class_name: Option<String>,
        namespace: Option<String>,
        signature: String,
    ) -> Self {
        Self {
            file_path,
            method_name,
            class_name,
            namespace,
            signature,
        }
    }

    /// Create a unique string representation
    pub fn to_unique_string(&self) -> String {
        let class_part = self.class_name.as_ref()
            .map(|c| format!("{}::", c))
            .unwrap_or_default();
        let namespace_part = self.namespace.as_ref()
            .map(|n| format!("{}::", n))
            .unwrap_or_default();

        format!("{}{}{}{}",
                namespace_part,
                class_part,
                self.method_name,
                self.signature)
    }

    /// Get display name for documentation
    pub fn display_name(&self) -> String {
        if let Some(class) = &self.class_name {
            format!("{}::{}", class, self.method_name)
        } else {
            self.method_name.clone()
        }
    }
}

/// Node in the call graph representing a method/function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallNode {
    pub signature: MethodSignature,
    pub line_range: (usize, usize),
    pub visibility: String,
    pub is_async: bool,
    pub documentation: Option<String>,
    /// Estimated complexity (lines, cyclomatic complexity, etc.)
    pub complexity_score: u32,
}

/// Edge in the call graph representing a method call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallEdge {
    /// Method making the call
    pub caller: MethodSignature,
    /// Method being called
    pub callee: MethodSignature,
    /// Line number where the call occurs
    pub call_site_line: usize,
    /// Type of call (direct, indirect, async, etc.)
    pub call_type: CallType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CallType {
    Direct,        // foo()
    Async,         // await foo()
    Callback,      // passed as callback
    Conditional,   // inside if/match
    Loop,          // inside loop
    Try,           // inside try/catch
}

/// Complete call graph for the codebase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallGraph {
    /// All nodes (methods) in the graph
    pub nodes: HashMap<MethodSignature, CallNode>,
    /// All edges (calls) in the graph
    pub edges: Vec<CallEdge>,
    /// Adjacency list for efficient traversal
    adjacency_list: HashMap<MethodSignature, Vec<MethodSignature>>,
    /// Reverse adjacency list (who calls this method)
    reverse_adjacency: HashMap<MethodSignature, Vec<MethodSignature>>,
    /// Detected cycles in the call graph
    pub cycles: Vec<Vec<MethodSignature>>,
}

impl CallGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            adjacency_list: HashMap::new(),
            reverse_adjacency: HashMap::new(),
            cycles: Vec::new(),
        }
    }

    /// Build call graph from parsed files
    pub fn build_from_files(files: &[ParsedFile]) -> Result<Self> {
        let mut graph = Self::new();

        // First pass: extract all methods/functions
        for file in files {
            graph.extract_methods_from_file(file)?;
        }

        println!("Extracted {} methods", graph.nodes.len());

        // Second pass: extract all method calls
        for file in files {
            graph.extract_calls_from_file(file)?;
        }

        println!("Found {} call edges", graph.edges.len());

        // Build adjacency lists
        graph.build_adjacency_lists();

        // Detect cycles
        graph.detect_cycles();

        Ok(graph)
    }

    /// Add a method node to the graph
    pub fn add_node(&mut self, node: CallNode) {
        self.nodes.insert(node.signature.clone(), node);
    }

    /// Add a call edge to the graph
    pub fn add_edge(&mut self, edge: CallEdge) {
        self.edges.push(edge);
    }

    /// Get methods that this method calls (outgoing edges)
    pub fn get_callees(&self, method: &MethodSignature) -> Vec<&MethodSignature> {
        self.adjacency_list.get(method)
            .map(|callees| callees.iter().collect())
            .unwrap_or_default()
    }

    /// Get methods that call this method (incoming edges)
    pub fn get_callers(&self, method: &MethodSignature) -> Vec<&MethodSignature> {
        self.reverse_adjacency.get(method)
            .map(|callers| callers.iter().collect())
            .unwrap_or_default()
    }

    /// Get in-degree (number of callers)
    pub fn in_degree(&self, method: &MethodSignature) -> usize {
        self.get_callers(method).len()
    }

    /// Get out-degree (number of callees)
    pub fn out_degree(&self, method: &MethodSignature) -> usize {
        self.get_callees(method).len()
    }

    /// Get all entry point candidates (in-degree = 0, out-degree > 0)
    pub fn get_entry_point_candidates(&self) -> Vec<&MethodSignature> {
        self.nodes.keys()
            .filter(|method| self.in_degree(method) == 0 && self.out_degree(method) > 0)
            .collect()
    }

    /// Get statistics about the call graph
    pub fn get_statistics(&self) -> CallGraphStats {
        CallGraphStats {
            total_methods: self.nodes.len(),
            total_calls: self.edges.len(),
            entry_points: self.get_entry_point_candidates().len(),
            cycles: self.cycles.len(),
            max_in_degree: self.nodes.keys().map(|m| self.in_degree(m)).max().unwrap_or(0),
            max_out_degree: self.nodes.keys().map(|m| self.out_degree(m)).max().unwrap_or(0),
        }
    }

    /// Extract all methods from a parsed file
    fn extract_methods_from_file(&mut self, file: &ParsedFile) -> Result<()> {
        for module in &file.modules {
            self.extract_methods_from_module(file, module, None)?;
        }
        Ok(())
    }

    /// Recursively extract methods from modules
    fn extract_methods_from_module(
        &mut self,
        file: &ParsedFile,
        module: &ParsedModule,
        parent_class: Option<&str>,
    ) -> Result<()> {
        match module.item_type.as_str() {
            "function" | "method" => {
                let signature = MethodSignature::new(
                    file.path.clone(),
                    module.name.clone(),
                    parent_class.map(|s| s.to_string()),
                    self.extract_namespace(&file.path),
                    module.signature.clone().unwrap_or_default(),
                );

                let node = CallNode {
                    signature,
                    line_range: module.line_range,
                    visibility: module.visibility.clone(),
                    is_async: module.signature.as_ref()
                        .map_or(false, |s| s.contains("async")),
                    documentation: module.docs.clone(),
                    complexity_score: self.estimate_complexity(module),
                };

                self.add_node(node);
            }
            "class" | "struct" | "impl" => {
                // Process methods within classes/structs
                for child in &module.children {
                    self.extract_methods_from_module(file, child, Some(&module.name))?;
                }
            }
            _ => {
                // Process any nested items
                for child in &module.children {
                    self.extract_methods_from_module(file, child, parent_class)?;
                }
            }
        }

        Ok(())
    }

    /// Extract method calls from a file - THIS IS THE KEY FIX
    fn extract_calls_from_file(&mut self, file: &ParsedFile) -> Result<()> {
        let lines: Vec<&str> = file.source_content.lines().collect();

        for (line_number, line) in lines.iter().enumerate() {
            let calls = self.extract_calls_from_line(line, &file.language);

            for call_name in calls {
                // Find which method this call is inside
                if let Some(containing_method) = self.find_containing_method(file, line_number + 1) {
                    // Try to resolve the call to a known method
                    if let Some(target_method) = self.resolve_method_call(file, &call_name, &containing_method) {
                        // CRITICAL: Only add edge if caller != callee
                        if containing_method != target_method {
                            let edge = CallEdge {
                                caller: containing_method,
                                callee: target_method,
                                call_site_line: line_number + 1,
                                call_type: self.detect_call_type(line),
                            };
                            self.add_edge(edge);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Extract method calls from a single line of code - ENHANCED VERSION
    fn extract_calls_from_line(&self, line: &str, language: &str) -> Vec<String> {
        let mut calls = Vec::new();
        let trimmed = line.trim();

        match language {
            "java" => {
                // Look for method calls: name() or object.name()
                if let Ok(re) = regex::Regex::new(r"(\w+)\.(\w+)\s*\(") {
                    for cap in re.captures_iter(trimmed) {
                        if let Some(method_name) = cap.get(2) {
                            let method_name = method_name.as_str();
                            if !self.is_language_keyword(method_name, language) {
                                calls.push(method_name.to_string());
                            }
                        }
                    }
                }

                // Also look for direct function calls: name()
                if let Ok(re) = regex::Regex::new(r"\b(\w+)\s*\(") {
                    for cap in re.captures_iter(trimmed) {
                        if let Some(method_name) = cap.get(1) {
                            let method_name = method_name.as_str();
                            if !self.is_language_keyword(method_name, language) &&
                                !self.is_java_builtin(method_name) {
                                calls.push(method_name.to_string());
                            }
                        }
                    }
                }
            }
            "rust" => {
                // Look for function calls: name() or name!(
                if let Ok(re) = regex::Regex::new(r"\b([a-zA-Z_][a-zA-Z0-9_]*)\s*[\(]") {
                    for cap in re.captures_iter(trimmed) {
                        if let Some(name) = cap.get(1) {
                            let method_name = name.as_str();
                            if !self.is_language_keyword(method_name, language) {
                                calls.push(method_name.to_string());
                            }
                        }
                    }
                }
            }
            "python" => {
                // Look for function calls: name() or object.name()
                if let Ok(re) = regex::Regex::new(r"\.?([a-zA-Z_][a-zA-Z0-9_]*)\s*\(") {
                    for cap in re.captures_iter(trimmed) {
                        if let Some(name) = cap.get(1) {
                            let method_name = name.as_str();
                            if !self.is_language_keyword(method_name, language) {
                                calls.push(method_name.to_string());
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        calls
    }

    /// Find which method contains a given line number
    fn find_containing_method(&self, file: &ParsedFile, line_number: usize) -> Option<MethodSignature> {
        for node in self.nodes.values() {
            if node.signature.file_path == file.path &&
                line_number >= node.line_range.0 &&
                line_number <= node.line_range.1 {
                return Some(node.signature.clone());
            }
        }
        None
    }

    /// Try to resolve a method call to a specific method signature - IMPROVED VERSION
    fn resolve_method_call(
        &self,
        _file: &ParsedFile,
        call_name: &str,
        _caller: &MethodSignature,
    ) -> Option<MethodSignature> {
        // Find all methods with matching names
        let mut candidates: Vec<&MethodSignature> = self.nodes.keys()
            .filter(|sig| sig.method_name == call_name)
            .collect();

        if candidates.is_empty() {
            return None;
        }

        // For now, return the first match
        // TODO: Improve with scope resolution and import analysis
        Some(candidates[0].clone())
    }

    /// Detect the type of method call based on context
    fn detect_call_type(&self, line: &str) -> CallType {
        let trimmed = line.trim();

        if trimmed.contains("await") {
            CallType::Async
        } else if trimmed.contains("if ") || trimmed.contains("switch ") {
            CallType::Conditional
        } else if trimmed.contains("for ") || trimmed.contains("while ") {
            CallType::Loop
        } else if trimmed.contains("try") || trimmed.contains("catch") {
            CallType::Try
        } else {
            CallType::Direct
        }
    }

    /// Check if a name is a language keyword
    fn is_language_keyword(&self, name: &str, language: &str) -> bool {
        let keywords: &[&str] = match language {
            "java" => &["if", "else", "for", "while", "switch", "case", "break", "continue", "return", "try", "catch", "finally", "throw", "new", "this", "super", "class", "interface", "public", "private", "protected", "static", "final"],
            "rust" => &["if", "else", "for", "while", "loop", "match", "let", "mut", "fn", "struct", "enum", "impl", "trait", "mod", "use", "pub", "return", "break", "continue"],
            "python" => &["if", "else", "for", "while", "def", "class", "import", "from", "return", "break", "continue", "try", "except", "finally", "raise", "with", "as"],
            _ => &[],
        };

        keywords.contains(&name)
    }

    /// Check if a name is a Java built-in method that we should ignore
    fn is_java_builtin(&self, name: &str) -> bool {
        let builtins = ["println", "print", "equals", "hashCode", "toString", "length", "size", "get", "put", "add", "remove"];
        builtins.contains(&name)
    }

    /// Extract namespace from file path
    fn extract_namespace(&self, file_path: &PathBuf) -> Option<String> {
        // Simple heuristic: use directory structure
        if let Some(parent) = file_path.parent() {
            if let Some(dir_name) = parent.file_name() {
                return Some(dir_name.to_string_lossy().to_string());
            }
        }
        None
    }

    /// Estimate complexity of a method
    fn estimate_complexity(&self, module: &ParsedModule) -> u32 {
        let line_count = module.line_range.1 - module.line_range.0;
        let signature_complexity = module.signature.as_ref()
            .map(|s| s.matches(',').count() as u32 + 1)
            .unwrap_or(1);

        line_count as u32 + signature_complexity
    }

    /// Build adjacency lists for efficient traversal
    fn build_adjacency_lists(&mut self) {
        self.adjacency_list.clear();
        self.reverse_adjacency.clear();

        for edge in &self.edges {
            // Forward adjacency (caller -> callee)
            self.adjacency_list
                .entry(edge.caller.clone())
                .or_insert_with(Vec::new)
                .push(edge.callee.clone());

            // Reverse adjacency (callee -> caller)
            self.reverse_adjacency
                .entry(edge.callee.clone())
                .or_insert_with(Vec::new)
                .push(edge.caller.clone());
        }
    }

    /// Detect cycles in the call graph using DFS
    fn detect_cycles(&mut self) {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut current_path = Vec::new();

        let node_keys: Vec<_> = self.nodes.keys().cloned().collect();

        for node in node_keys {
            if !visited.contains(&node) {
                self.dfs_cycle_detection(
                    &node,
                    &mut visited,
                    &mut rec_stack,
                    &mut current_path,
                );
            }
        }
    }

    /// DFS helper for cycle detection
    fn dfs_cycle_detection(
        &mut self,
        node: &MethodSignature,
        visited: &mut HashSet<MethodSignature>,
        rec_stack: &mut HashSet<MethodSignature>,
        current_path: &mut Vec<MethodSignature>,
    ) {
        visited.insert(node.clone());
        rec_stack.insert(node.clone());
        current_path.push(node.clone());

        let callees: Vec<_> = self.adjacency_list.get(node)
            .map(|v| v.clone())
            .unwrap_or_default();

        for callee in callees {
            if !visited.contains(&callee) {
                self.dfs_cycle_detection(&callee, visited, rec_stack, current_path);
            } else if rec_stack.contains(&callee) {
                if let Some(cycle_start) = current_path.iter().position(|n| n == &callee) {
                    let cycle = current_path[cycle_start..].to_vec();
                    self.cycles.push(cycle);
                }
            }
        }

        rec_stack.remove(node);
        current_path.pop();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallGraphStats {
    pub total_methods: usize,
    pub total_calls: usize,
    pub entry_points: usize,
    pub cycles: usize,
    pub max_in_degree: usize,
    pub max_out_degree: usize,
}

impl Default for CallGraph {
    fn default() -> Self {
        Self::new()
    }
}