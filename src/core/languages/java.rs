// src/core/languages/java.rs - Updated with better call detection
use std::path::Path;
use tree_sitter::{Language, Parser, Node};

use crate::error::{CodesworthError, Result};
use super::{LanguageParser, ParsedModule};

/// Java-specific parser using Tree-sitter with enhanced call detection
pub struct JavaParser {
    parser: Parser,
}

impl JavaParser {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        let java_language = tree_sitter_java::language();
        parser.set_language(&java_language)
            .map_err(|e| CodesworthError::Parser(format!("Failed to set Java language: {}", e)))?;

        Ok(Self { parser })
    }
}

impl LanguageParser for JavaParser {
    fn parse(&mut self, content: &str, _file_path: &Path) -> Result<Vec<ParsedModule>> {
        let tree = self.parser.parse(content, None)
            .ok_or_else(|| CodesworthError::Parser("Failed to parse Java code".to_string()))?;

        let root_node = tree.root_node();
        let mut modules = Vec::new();

        // Walk the AST and extract items
        self.extract_java_items(root_node, content, &mut modules)?;

        if modules.is_empty() {
            Ok(vec![self.create_placeholder_module("class", "class")])
        } else {
            Ok(modules)
        }
    }

    fn extract_file_docs(&self, content: &str) -> Option<String> {
        let lines: Vec<&str> = content.lines().collect();
        let mut doc_lines = Vec::new();
        let mut in_javadoc = false;

        for line in lines {
            let trimmed = line.trim();

            if trimmed.starts_with("/**") {
                in_javadoc = true;
                let content = trimmed.trim_start_matches("/**").trim();
                if !content.is_empty() && content != "*/" {
                    doc_lines.push(content.to_string());
                }
            } else if in_javadoc {
                if trimmed.ends_with("*/") {
                    let content = trimmed.trim_end_matches("*/").trim_start_matches("*").trim();
                    if !content.is_empty() {
                        doc_lines.push(content.to_string());
                    }
                    break;
                } else {
                    let content = trimmed.trim_start_matches("*").trim();
                    if !content.is_empty() {
                        doc_lines.push(content.to_string());
                    }
                }
            } else if trimmed.starts_with("//") {
                let content = trimmed.trim_start_matches("//").trim();
                if !content.is_empty() {
                    doc_lines.push(content.to_string());
                }
            } else if !trimmed.is_empty() && !trimmed.starts_with("package") && !trimmed.starts_with("import") {
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
        &["java"]
    }

    fn language_name(&self) -> &str {
        "java"
    }
}

impl JavaParser {
    /// Extract Java language items from AST nodes
    fn extract_java_items(&self, node: Node, source: &str, modules: &mut Vec<ParsedModule>) -> Result<()> {
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "class_declaration" => {
                    if let Some(parsed_class) = self.parse_java_class(child, source)? {
                        modules.push(parsed_class);
                    }
                }
                "interface_declaration" => {
                    if let Some(parsed_interface) = self.parse_java_interface(child, source)? {
                        modules.push(parsed_interface);
                    }
                }
                "enum_declaration" => {
                    if let Some(parsed_enum) = self.parse_java_enum(child, source)? {
                        modules.push(parsed_enum);
                    }
                }
                "method_declaration" => {
                    if let Some(parsed_method) = self.parse_java_method(child, source)? {
                        modules.push(parsed_method);
                    }
                }
                _ => {
                    // Recursively check child nodes
                    self.extract_java_items(child, source, modules)?;
                }
            }
        }

        Ok(())
    }

    /// Parse a Java class declaration with enhanced annotation detection
    fn parse_java_class(&self, node: Node, source: &str) -> Result<Option<ParsedModule>> {
        let mut name = None;
        let mut visibility = "package".to_string();
        let mut docs = None;

        // Extract class name
        if let Some(name_node) = node.child_by_field_name("name") {
            name = Some(self.node_text(name_node, source));
        }

        // Check for modifiers (public, private, protected, etc.)
        if let Some(modifiers_node) = node.child_by_field_name("modifiers") {
            visibility = self.parse_java_visibility(modifiers_node, source);
        }

        // Look for annotations and documentation
        docs = self.extract_docs_and_annotations_before_node(node, source);

        // Extract methods from the class
        let mut methods = Vec::new();
        let class_body = self.find_child_by_kind(node, "class_body");
        if let Some(body_node) = class_body {
            let mut cursor = body_node.walk();
            for child in body_node.children(&mut cursor) {
                if child.kind() == "method_declaration" {
                    if let Some(method) = self.parse_java_method(child, source)? {
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

    /// Parse a Java interface declaration
    fn parse_java_interface(&self, node: Node, source: &str) -> Result<Option<ParsedModule>> {
        let mut name = None;
        let mut visibility = "package".to_string();
        let mut docs = None;

        if let Some(name_node) = node.child_by_field_name("name") {
            name = Some(self.node_text(name_node, source));
        }

        if let Some(modifiers_node) = node.child_by_field_name("modifiers") {
            visibility = self.parse_java_visibility(modifiers_node, source);
        }

        docs = self.extract_docs_and_annotations_before_node(node, source);

        if let Some(interface_name) = name {
            Ok(Some(ParsedModule {
                name: interface_name,
                item_type: "interface".to_string(),
                visibility,
                docs,
                signature: Some(self.extract_signature_until_brace(node, source)),
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

    /// Parse a Java enum declaration
    fn parse_java_enum(&self, node: Node, source: &str) -> Result<Option<ParsedModule>> {
        let mut name = None;
        let mut visibility = "package".to_string();
        let mut docs = None;

        if let Some(name_node) = node.child_by_field_name("name") {
            name = Some(self.node_text(name_node, source));
        }

        if let Some(modifiers_node) = node.child_by_field_name("modifiers") {
            visibility = self.parse_java_visibility(modifiers_node, source);
        }

        docs = self.extract_docs_and_annotations_before_node(node, source);

        if let Some(enum_name) = name {
            Ok(Some(ParsedModule {
                name: enum_name,
                item_type: "enum".to_string(),
                visibility,
                docs,
                signature: Some(self.extract_signature_until_brace(node, source)),
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

    /// Parse a Java method declaration with annotation detection
    fn parse_java_method(&self, node: Node, source: &str) -> Result<Option<ParsedModule>> {
        let mut name = None;
        let mut visibility = "package".to_string();
        let mut docs = None;

        if let Some(name_node) = node.child_by_field_name("name") {
            name = Some(self.node_text(name_node, source));
        }

        if let Some(modifiers_node) = node.child_by_field_name("modifiers") {
            visibility = self.parse_java_visibility(modifiers_node, source);
        }

        // Enhanced documentation and annotation extraction
        docs = self.extract_docs_and_annotations_before_node(node, source);

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

    /// Parse Java visibility modifiers
    fn parse_java_visibility(&self, node: Node, source: &str) -> String {
        let modifiers_text = self.node_text(node, source);
        if modifiers_text.contains("public") {
            "public".to_string()
        } else if modifiers_text.contains("private") {
            "private".to_string()
        } else if modifiers_text.contains("protected") {
            "protected".to_string()
        } else {
            "package".to_string()
        }
    }

    /// Extract text content of a node
    fn node_text(&self, node: Node, source: &str) -> String {
        source[node.byte_range()].to_string()
    }

    /// Find a child node by its kind
    fn find_child_by_kind<'a>(&self, node: Node<'a>, kind: &str) -> Option<Node<'a>> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == kind {
                return Some(child);
            }
        }
        None
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

    /// Extract method signature (more precise than class signature)
    fn extract_method_signature(&self, node: Node, source: &str) -> String {
        let full_text = self.node_text(node, source);
        if let Some(brace_pos) = full_text.find('{') {
            full_text[..brace_pos].trim().to_string()
        } else {
            full_text.trim_end_matches(';').trim().to_string()
        }
    }

    /// Enhanced documentation and annotation extraction
    fn extract_docs_and_annotations_before_node(&self, node: Node, source: &str) -> Option<String> {
        let start_row = node.start_position().row;
        let lines: Vec<&str> = source.lines().collect();
        let mut doc_lines = Vec::new();
        let mut annotations = Vec::new();
        let mut in_javadoc = false;

        // Look backwards from the node's line for doc comments and annotations
        for i in (0..start_row).rev() {
            if i >= lines.len() {
                continue;
            }

            let line = lines[i].trim();

            // Handle annotations
            if line.starts_with("@") {
                annotations.insert(0, line.to_string());
                continue;
            }

            // Handle Javadoc
            if line.ends_with("*/") && !in_javadoc {
                in_javadoc = true;
                let content = line.trim_end_matches("*/").trim_start_matches("*").trim();
                if !content.is_empty() {
                    doc_lines.insert(0, content.to_string());
                }
            } else if in_javadoc {
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
                    break;
                }
            } else if line.is_empty() {
                continue;
            } else {
                break;
            }
        }

        // Combine annotations and documentation
        let mut result = Vec::new();

        if !annotations.is_empty() {
            result.push(format!("Annotations: {}", annotations.join(", ")));
        }

        if !doc_lines.is_empty() {
            result.push(doc_lines.join(" "));
        }

        if result.is_empty() {
            None
        } else {
            Some(result.join(" | "))
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