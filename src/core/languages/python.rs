use std::path::Path;
use tree_sitter::{Language, Parser, Node};

use crate::error::{CodesworthError, Result};
use super::{LanguageParser, ParsedModule};

/// Python-specific parser using Tree-sitter
pub struct PythonParser {
    parser: Parser,
}

impl PythonParser {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        let python_language = tree_sitter_python::language();
        parser.set_language(&python_language)
            .map_err(|e| CodesworthError::Parser(format!("Failed to set Python language: {}", e)))?;

        Ok(Self { parser })
    }
}

impl LanguageParser for PythonParser {
    fn parse(&mut self, content: &str, _file_path: &Path) -> Result<Vec<ParsedModule>> {
        let tree = self.parser.parse(content, None)
            .ok_or_else(|| CodesworthError::Parser("Failed to parse Python code".to_string()))?;

        let root_node = tree.root_node();
        let mut modules = Vec::new();

        // Walk the AST and extract items
        self.extract_python_items(root_node, content, &mut modules)?;

        if modules.is_empty() {
            Ok(vec![self.create_placeholder_module("module", "module")])
        } else {
            Ok(modules)
        }
    }

    fn extract_file_docs(&self, content: &str) -> Option<String> {
        let lines: Vec<&str> = content.lines().collect();
        let mut doc_lines = Vec::new();
        let mut in_docstring = false;
        let mut docstring_delimiter = "";

        for line in lines {
            let trimmed = line.trim();

            if !in_docstring {
                if trimmed.starts_with("\"\"\"") || trimmed.starts_with("'''") {
                    in_docstring = true;
                    docstring_delimiter = if trimmed.starts_with("\"\"\"") { "\"\"\"" } else { "'''" };

                    // Extract content after opening delimiter
                    let content = trimmed.trim_start_matches(docstring_delimiter);
                    if trimmed.ends_with(docstring_delimiter) && content.len() > 0 {
                        // Single line docstring
                        let content = content.trim_end_matches(docstring_delimiter).trim();
                        if !content.is_empty() {
                            doc_lines.push(content.to_string());
                        }
                        break;
                    } else if !content.trim().is_empty() {
                        doc_lines.push(content.trim().to_string());
                    }
                } else if trimmed.starts_with("#") {
                    // Comment line at file level
                    let content = trimmed.trim_start_matches("#").trim();
                    if !content.is_empty() {
                        doc_lines.push(content.to_string());
                    }
                } else if !trimmed.is_empty() && !trimmed.starts_with("import") && !trimmed.starts_with("from") {
                    // Hit code, stop looking for file-level docs
                    break;
                }
            } else {
                if trimmed.ends_with(docstring_delimiter) {
                    let content = trimmed.trim_end_matches(docstring_delimiter).trim();
                    if !content.is_empty() {
                        doc_lines.push(content.to_string());
                    }
                    break;
                } else {
                    if !trimmed.is_empty() {
                        doc_lines.push(trimmed.to_string());
                    }
                }
            }
        }

        if doc_lines.is_empty() {
            None
        } else {
            Some(doc_lines.join(" "))
        }
    }

    fn file_extensions(&self) -> &[&str] {
        &["py"]
    }

    fn language_name(&self) -> &str {
        "python"
    }
}

impl PythonParser {
    /// Extract Python language items from AST nodes
    fn extract_python_items(&self, node: Node, source: &str, modules: &mut Vec<ParsedModule>) -> Result<()> {
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "class_definition" => {
                    if let Some(parsed_class) = self.parse_python_class(child, source)? {
                        modules.push(parsed_class);
                    }
                }
                "function_definition" => {
                    if let Some(parsed_function) = self.parse_python_function(child, source)? {
                        modules.push(parsed_function);
                    }
                }
                _ => {
                    // Recursively check child nodes
                    self.extract_python_items(child, source, modules)?;
                }
            }
        }

        Ok(())
    }

    /// Parse a Python class definition
    fn parse_python_class(&self, node: Node, source: &str) -> Result<Option<ParsedModule>> {
        let mut name = None;
        let visibility = "public".to_string(); // Python doesn't have explicit visibility
        let mut docs = None;

        // Extract class name
        if let Some(name_node) = node.child_by_field_name("name") {
            name = Some(self.node_text(name_node, source));
        }

        // Look for docstring
        docs = self.extract_docstring_from_body(node, source);

        // Extract methods from the class
        let mut methods = Vec::new();
        if let Some(body_node) = node.child_by_field_name("body") {
            let mut cursor = body_node.walk();
            for child in body_node.children(&mut cursor) {
                if child.kind() == "function_definition" {
                    if let Some(method) = self.parse_python_function(child, source)? {
                        methods.push(method);
                    }
                }
            }
        }

        if let Some(class_name) = name {
            Ok(Some(ParsedModule {
                name: class_name,
                item_type: "class".to_string(),
                visibility,
                docs,
                signature: Some(self.extract_signature_until_colon(node, source)),
                line_range: (
                    node.start_position().row + 1,
                    node.end_position().row + 1
                ),
                children: methods,
            }))
        } else {
            Ok(None)
        }
    }

    /// Parse a Python function definition
    fn parse_python_function(&self, node: Node, source: &str) -> Result<Option<ParsedModule>> {
        let mut name = None;
        let mut visibility = "public".to_string();
        let mut docs = None;

        // Extract function name
        if let Some(name_node) = node.child_by_field_name("name") {
            let function_name = self.node_text(name_node, source);
            name = Some(function_name.clone());

            // Python convention: methods starting with _ are "private"
            if function_name.starts_with("__") && function_name.ends_with("__") {
                visibility = "special".to_string(); // Magic methods
            } else if function_name.starts_with("_") {
                visibility = "private".to_string();
            }
        }

        // Look for docstring
        docs = self.extract_docstring_from_body(node, source);

        if let Some(function_name) = name {
            Ok(Some(ParsedModule {
                name: function_name,
                item_type: "function".to_string(),
                visibility,
                docs,
                signature: Some(self.extract_function_signature(node, source)),
                line_range: (
                    node.start_position().row + 1,
                    node.end_position().row + 1
                ),
                children: vec![],
            }))
        } else {
            Ok(None)
        }
    }

    /// Extract text content of a node
    fn node_text(&self, node: Node, source: &str) -> String {
        source[node.byte_range()].to_string()
    }

    /// Extract signature until colon (for classes)
    fn extract_signature_until_colon(&self, node: Node, source: &str) -> String {
        let full_text = self.node_text(node, source);
        let lines: Vec<&str> = full_text.lines().collect();
        let mut signature_lines = Vec::new();

        for line in lines {
            let trimmed = line.trim();
            if trimmed.contains(':') && !trimmed.starts_with("#") {
                if let Some(colon_pos) = line.find(':') {
                    let before_colon = line[..colon_pos].trim();
                    if !before_colon.is_empty() {
                        signature_lines.push(before_colon);
                    }
                }
                break;
            } else {
                signature_lines.push(trimmed);
            }
        }

        signature_lines.join(" ").trim().to_string()
    }

    /// Extract function signature (including def, name, parameters)
    fn extract_function_signature(&self, node: Node, source: &str) -> String {
        let full_text = self.node_text(node, source);
        if let Some(colon_pos) = full_text.find(':') {
            full_text[..colon_pos].trim().to_string()
        } else {
            full_text.lines().next().unwrap_or("").trim().to_string()
        }
    }

    /// Extract docstring from function/class body
    fn extract_docstring_from_body(&self, node: Node, source: &str) -> Option<String> {
        if let Some(body_node) = node.child_by_field_name("body") {
            let mut cursor = body_node.walk();
            for child in body_node.children(&mut cursor) {
                if child.kind() == "expression_statement" {
                    // Look for string literal as first statement (docstring)
                    let mut expr_cursor = child.walk();
                    for expr_child in child.children(&mut expr_cursor) {
                        if expr_child.kind() == "string" {
                            let docstring = self.node_text(expr_child, source);
                            // Clean up the docstring (remove quotes and extra whitespace)
                            let cleaned = docstring
                                .trim_matches('"')
                                .trim_matches('\'')
                                .trim_start_matches("\"\"\"")
                                .trim_end_matches("\"\"\"")
                                .trim_start_matches("'''")
                                .trim_end_matches("'''")
                                .trim();
                            if !cleaned.is_empty() {
                                return Some(cleaned.to_string());
                            }
                        }
                    }
                    break; // Only check first statement
                }
            }
        }
        None
    }

    /// Create a placeholder module for testing
    fn create_placeholder_module(&self, name: &str, item_type: &str) -> ParsedModule {
        ParsedModule {
            name: name.to_string(),
            item_type: item_type.to_string(),
            visibility: "public".to_string(),
            docs: Some(format!("Documentation for {}", name)),
            signature: None,
            line_range: (1, 10),
            children: vec![],
        }
    }
}