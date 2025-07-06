use std::path::Path;
use tree_sitter::{Language, Parser, Node};

use crate::error::{CodesworthError, Result};
use super::{LanguageParser, ParsedModule};

/// JavaScript/TypeScript-specific parser using Tree-sitter
pub struct JavaScriptParser {
    parser: Parser,
}

impl JavaScriptParser {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        let javascript_language = tree_sitter_javascript::language();
        parser.set_language(&javascript_language)
            .map_err(|e| CodesworthError::Parser(format!("Failed to set JavaScript language: {}", e)))?;

        Ok(Self { parser })
    }
}

impl LanguageParser for JavaScriptParser {
    fn parse(&mut self, content: &str, _file_path: &Path) -> Result<Vec<ParsedModule>> {
        let tree = self.parser.parse(content, None)
            .ok_or_else(|| CodesworthError::Parser("Failed to parse JavaScript code".to_string()))?;

        let root_node = tree.root_node();
        let mut modules = Vec::new();

        // Walk the AST and extract items
        self.extract_javascript_items(root_node, content, &mut modules)?;

        if modules.is_empty() {
            Ok(vec![self.create_placeholder_module("module", "module")])
        } else {
            Ok(modules)
        }
    }

    fn extract_file_docs(&self, content: &str) -> Option<String> {
        let lines: Vec<&str> = content.lines().collect();
        let mut doc_lines = Vec::new();
        let mut in_jsdoc = false;

        for line in lines {
            let trimmed = line.trim();

            if trimmed.starts_with("/**") {
                in_jsdoc = true;
                // Extract content after /**
                let content = trimmed.trim_start_matches("/**").trim();
                if !content.is_empty() && content != "*/" {
                    doc_lines.push(content.to_string());
                }
            } else if in_jsdoc {
                if trimmed.ends_with("*/") {
                    // Extract content before */
                    let content = trimmed.trim_end_matches("*/").trim_start_matches("*").trim();
                    if !content.is_empty() {
                        doc_lines.push(content.to_string());
                    }
                    break;
                } else {
                    // Extract content from middle of jsdoc
                    let content = trimmed.trim_start_matches("*").trim();
                    if !content.is_empty() {
                        doc_lines.push(content.to_string());
                    }
                }
            } else if trimmed.starts_with("//") {
                // Single line comment at file level
                let content = trimmed.trim_start_matches("//").trim();
                if !content.is_empty() {
                    doc_lines.push(content.to_string());
                }
            } else if !trimmed.is_empty() && !trimmed.starts_with("import") && !trimmed.starts_with("export") {
                // Hit code, stop looking for file-level docs
                break;
            }
        }

        if doc_lines.is_empty() {
            None
        } else {
            Some(doc_lines.join(" "))
        }
    }

    fn file_extensions(&self) -> &[&str] {
        &["js", "jsx", "ts", "tsx"]
    }

    fn language_name(&self) -> &str {
        "javascript"
    }
}

impl JavaScriptParser {
    /// Extract JavaScript language items from AST nodes
    fn extract_javascript_items(&self, node: Node, source: &str, modules: &mut Vec<ParsedModule>) -> Result<()> {
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "class_declaration" => {
                    if let Some(parsed_class) = self.parse_javascript_class(child, source)? {
                        modules.push(parsed_class);
                    }
                }
                "function_declaration" => {
                    if let Some(parsed_function) = self.parse_javascript_function(child, source)? {
                        modules.push(parsed_function);
                    }
                }
                "arrow_function" => {
                    if let Some(parsed_arrow) = self.parse_javascript_arrow_function(child, source)? {
                        modules.push(parsed_arrow);
                    }
                }
                "variable_declaration" => {
                    // Check if this is a function assignment
                    if let Some(parsed_var) = self.parse_javascript_variable(child, source)? {
                        modules.push(parsed_var);
                    }
                }
                "export_statement" => {
                    // Handle exports
                    self.extract_javascript_items(child, source, modules)?;
                }
                _ => {
                    // Recursively check child nodes
                    self.extract_javascript_items(child, source, modules)?;
                }
            }
        }

        Ok(())
    }

    /// Parse a JavaScript class declaration
    fn parse_javascript_class(&self, node: Node, source: &str) -> Result<Option<ParsedModule>> {
        let mut name = None;
        let visibility = "public".to_string(); // JavaScript doesn't have explicit visibility
        let mut docs = None;

        // Extract class name
        if let Some(name_node) = node.child_by_field_name("name") {
            name = Some(self.node_text(name_node, source));
        }

        // Look for JSDoc comments
        docs = self.extract_docs_before_node(node, source);

        // Extract methods from the class
        let mut methods = Vec::new();
        if let Some(body_node) = node.child_by_field_name("body") {
            let mut cursor = body_node.walk();
            for child in body_node.children(&mut cursor) {
                if child.kind() == "method_definition" {
                    if let Some(method) = self.parse_javascript_method(child, source)? {
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
                signature: Some(self.extract_signature_until_brace(node, source)),
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

    /// Parse a JavaScript function declaration
    fn parse_javascript_function(&self, node: Node, source: &str) -> Result<Option<ParsedModule>> {
        let mut name = None;
        let visibility = "public".to_string();
        let mut docs = None;

        // Extract function name
        if let Some(name_node) = node.child_by_field_name("name") {
            name = Some(self.node_text(name_node, source));
        }

        // Look for JSDoc comments
        docs = self.extract_docs_before_node(node, source);

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

    /// Parse a JavaScript arrow function
    fn parse_javascript_arrow_function(&self, node: Node, source: &str) -> Result<Option<ParsedModule>> {
        let name = "arrow_function".to_string(); // Generic name for arrow functions
        let visibility = "public".to_string();
        let docs = self.extract_docs_before_node(node, source);

        Ok(Some(ParsedModule {
            name,
            item_type: "arrow_function".to_string(),
            visibility,
            docs,
            signature: Some(self.extract_arrow_function_signature(node, source)),
            line_range: (
                node.start_position().row + 1,
                node.end_position().row + 1
            ),
            children: vec![],
        }))
    }

    /// Parse a JavaScript variable declaration (could be function assignment)
    fn parse_javascript_variable(&self, node: Node, source: &str) -> Result<Option<ParsedModule>> {
        // Check if this variable is assigned a function
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "variable_declarator" {
                if let Some(name_node) = child.child_by_field_name("name") {
                    if let Some(value_node) = child.child_by_field_name("value") {
                        if value_node.kind() == "arrow_function" || value_node.kind() == "function_expression" {
                            let name = self.node_text(name_node, source);
                            let docs = self.extract_docs_before_node(node, source);

                            return Ok(Some(ParsedModule {
                                name,
                                item_type: "function".to_string(),
                                visibility: "public".to_string(),
                                docs,
                                signature: Some(self.extract_variable_function_signature(child, source)),
                                line_range: (
                                    node.start_position().row + 1,
                                    node.end_position().row + 1
                                ),
                                children: vec![],
                            }));
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Parse a JavaScript method definition
    fn parse_javascript_method(&self, node: Node, source: &str) -> Result<Option<ParsedModule>> {
        let mut name = None;
        let mut visibility = "public".to_string();
        let mut docs = None;

        // Extract method name
        if let Some(name_node) = node.child_by_field_name("name") {
            let method_name = self.node_text(name_node, source);
            name = Some(method_name.clone());

            // JavaScript convention: methods starting with _ are "private"
            if method_name.starts_with("_") {
                visibility = "private".to_string();
            }
        }

        // Look for JSDoc comments
        docs = self.extract_docs_before_node(node, source);

        if let Some(method_name) = name {
            Ok(Some(ParsedModule {
                name: method_name,
                item_type: "method".to_string(),
                visibility,
                docs,
                signature: Some(self.extract_method_signature(node, source)),
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

    /// Extract signature until opening brace
    fn extract_signature_until_brace(&self, node: Node, source: &str) -> String {
        let full_text = self.node_text(node, source);
        let lines: Vec<&str> = full_text.lines().collect();
        let mut signature_lines = Vec::new();

        for line in lines {
            let trimmed = line.trim();
            if trimmed.contains('{') && !trimmed.starts_with("//") && !trimmed.starts_with("/*") {
                if let Some(brace_pos) = line.find('{') {
                    let before_brace = line[..brace_pos].trim();
                    if !before_brace.is_empty() {
                        signature_lines.push(before_brace);
                    }
                }
                break;
            } else {
                signature_lines.push(trimmed);
            }
        }

        signature_lines.join(" ").trim().to_string()
    }

    /// Extract function signature
    fn extract_function_signature(&self, node: Node, source: &str) -> String {
        let full_text = self.node_text(node, source);
        if let Some(brace_pos) = full_text.find('{') {
            full_text[..brace_pos].trim().to_string()
        } else {
            full_text.lines().next().unwrap_or("").trim().to_string()
        }
    }

    /// Extract arrow function signature
    fn extract_arrow_function_signature(&self, node: Node, source: &str) -> String {
        let full_text = self.node_text(node, source);
        if let Some(arrow_pos) = full_text.find("=>") {
            let before_arrow = full_text[..arrow_pos + 2].trim();
            format!("{} => ...", before_arrow.trim_end_matches("=>").trim())
        } else {
            full_text.trim().to_string()
        }
    }

    /// Extract variable function signature
    fn extract_variable_function_signature(&self, node: Node, source: &str) -> String {
        let full_text = self.node_text(node, source);
        full_text.trim().to_string()
    }

    /// Extract method signature
    fn extract_method_signature(&self, node: Node, source: &str) -> String {
        let full_text = self.node_text(node, source);
        if let Some(brace_pos) = full_text.find('{') {
            full_text[..brace_pos].trim().to_string()
        } else {
            full_text.trim().to_string()
        }
    }

    /// Extract JSDoc comments before a node
    fn extract_docs_before_node(&self, node: Node, source: &str) -> Option<String> {
        let start_row = node.start_position().row;
        let lines: Vec<&str> = source.lines().collect();
        let mut doc_lines = Vec::new();
        let mut in_jsdoc = false;

        // Look backwards from the node's line for JSDoc comments
        for i in (0..start_row).rev() {
            if i >= lines.len() {
                continue;
            }

            let line = lines[i].trim();

            if line.ends_with("*/") && !in_jsdoc {
                in_jsdoc = true;
                let content = line.trim_end_matches("*/").trim_start_matches("*").trim();
                if !content.is_empty() {
                    doc_lines.insert(0, content.to_string());
                }
            } else if in_jsdoc {
                if line.starts_with("/**") {
                    let content = line.trim_start_matches("/**").trim();
                    if !content.is_empty() && content != "*/" {
                        doc_lines.insert(0, content.to_string());
                    }
                    break;
                } else {
                    let content = line.trim_start_matches("*").trim();
                    if !content.is_empty() {
                        doc_lines.insert(0, content.to_string());
                    }
                }
            } else if line.starts_with("//") {
                let content = line.trim_start_matches("//").trim();
                if !content.is_empty() {
                    doc_lines.insert(0, content.to_string());
                } else {
                    break; // Empty comment line, stop looking
                }
            } else if line.is_empty() {
                // Allow empty lines in doc blocks
                continue;
            } else {
                // Hit code, stop looking
                break;
            }
        }

        if doc_lines.is_empty() {
            None
        } else {
            Some(doc_lines.join(" "))
        }
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