use std::path::Path;
use tree_sitter::{Language, Parser, Node};

use crate::error::{CodesworthError, Result};
use super::{LanguageParser, ParsedModule};

/// C#-specific parser using Tree-sitter
pub struct CSharpParser {
    parser: Parser,
}

impl CSharpParser {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        let csharp_language = tree_sitter_c_sharp::language();
        parser.set_language(&csharp_language)
            .map_err(|e| CodesworthError::Parser(format!("Failed to set C# language: {}", e)))?;

        Ok(Self { parser })
    }
}

impl LanguageParser for CSharpParser {
    fn parse(&mut self, content: &str, _file_path: &Path) -> Result<Vec<ParsedModule>> {
        let tree = self.parser.parse(content, None)
            .ok_or_else(|| CodesworthError::Parser("Failed to parse C# code".to_string()))?;

        let root_node = tree.root_node();
        let mut modules = Vec::new();

        // Walk the AST and extract items
        self.extract_csharp_items(root_node, content, &mut modules)?;

        if modules.is_empty() {
            Ok(vec![self.create_placeholder_module("class", "class")])
        } else {
            Ok(modules)
        }
    }

    fn extract_file_docs(&self, content: &str) -> Option<String> {
        let lines: Vec<&str> = content.lines().collect();
        let mut doc_lines = Vec::new();
        let mut in_xml_doc = false;

        for line in lines {
            let trimmed = line.trim();

            if trimmed.starts_with("///") {
                in_xml_doc = true;
                let content = trimmed.trim_start_matches("///").trim();
                if !content.is_empty() {
                    doc_lines.push(content.to_string());
                }
            } else if in_xml_doc && trimmed.starts_with("//") {
                let content = trimmed.trim_start_matches("//").trim();
                if !content.is_empty() {
                    doc_lines.push(content.to_string());
                } else {
                    break; // Empty comment line, end of doc block
                }
            } else if trimmed.starts_with("//") {
                // Single line comment at file level
                let content = trimmed.trim_start_matches("//").trim();
                if !content.is_empty() {
                    doc_lines.push(content.to_string());
                }
            } else if !trimmed.is_empty() && !trimmed.starts_with("using") && !trimmed.starts_with("namespace") {
                // Hit code, stop looking for file-level docs
                break;
            } else if in_xml_doc && !trimmed.is_empty() && !trimmed.starts_with("//") {
                // Hit code after XML doc, we're done
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
        &["cs"]
    }

    fn language_name(&self) -> &str {
        "csharp"
    }
}

impl CSharpParser {
    /// Extract C# language items from AST nodes
    fn extract_csharp_items(&self, node: Node, source: &str, modules: &mut Vec<ParsedModule>) -> Result<()> {
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "class_declaration" => {
                    if let Some(parsed_class) = self.parse_csharp_class(child, source)? {
                        modules.push(parsed_class);
                    }
                }
                "interface_declaration" => {
                    if let Some(parsed_interface) = self.parse_csharp_interface(child, source)? {
                        modules.push(parsed_interface);
                    }
                }
                "struct_declaration" => {
                    if let Some(parsed_struct) = self.parse_csharp_struct(child, source)? {
                        modules.push(parsed_struct);
                    }
                }
                "enum_declaration" => {
                    if let Some(parsed_enum) = self.parse_csharp_enum(child, source)? {
                        modules.push(parsed_enum);
                    }
                }
                "method_declaration" => {
                    if let Some(parsed_method) = self.parse_csharp_method(child, source)? {
                        modules.push(parsed_method);
                    }
                }
                "namespace_declaration" => {
                    if let Some(parsed_namespace) = self.parse_csharp_namespace(child, source)? {
                        modules.push(parsed_namespace);
                    }
                }
                _ => {
                    // Recursively check child nodes
                    self.extract_csharp_items(child, source, modules)?;
                }
            }
        }

        Ok(())
    }

    /// Parse a C# class declaration
    fn parse_csharp_class(&self, node: Node, source: &str) -> Result<Option<ParsedModule>> {
        let mut name = None;
        let mut visibility = "internal".to_string(); // C# default visibility
        let mut docs = None;

        // Extract class name
        if let Some(name_node) = node.child_by_field_name("name") {
            name = Some(self.node_text(name_node, source));
        }

        // Check for modifiers (public, private, protected, etc.)
        if let Some(modifiers_node) = node.child_by_field_name("modifiers") {
            visibility = self.parse_csharp_visibility(modifiers_node, source);
        }

        // Look for XML documentation comments
        docs = self.extract_docs_before_node(node, source);

        // Extract methods from the class
        let mut methods = Vec::new();
        if let Some(body_node) = node.child_by_field_name("body") {
            let mut cursor = body_node.walk();
            for child in body_node.children(&mut cursor) {
                if child.kind() == "method_declaration" {
                    if let Some(method) = self.parse_csharp_method(child, source)? {
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

    /// Parse a C# interface declaration
    fn parse_csharp_interface(&self, node: Node, source: &str) -> Result<Option<ParsedModule>> {
        let mut name = None;
        let mut visibility = "internal".to_string();
        let mut docs = None;

        if let Some(name_node) = node.child_by_field_name("name") {
            name = Some(self.node_text(name_node, source));
        }

        if let Some(modifiers_node) = node.child_by_field_name("modifiers") {
            visibility = self.parse_csharp_visibility(modifiers_node, source);
        }

        docs = self.extract_docs_before_node(node, source);

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
                children: vec![], // TODO: Parse interface methods
            }))
        } else {
            Ok(None)
        }
    }

    /// Parse a C# struct declaration
    fn parse_csharp_struct(&self, node: Node, source: &str) -> Result<Option<ParsedModule>> {
        let mut name = None;
        let mut visibility = "internal".to_string();
        let mut docs = None;

        if let Some(name_node) = node.child_by_field_name("name") {
            name = Some(self.node_text(name_node, source));
        }

        if let Some(modifiers_node) = node.child_by_field_name("modifiers") {
            visibility = self.parse_csharp_visibility(modifiers_node, source);
        }

        docs = self.extract_docs_before_node(node, source);

        if let Some(struct_name) = name {
            Ok(Some(ParsedModule {
                name: struct_name,
                item_type: "struct".to_string(),
                visibility,
                docs,
                signature: Some(self.extract_signature_until_brace(node, source)),
                line_range: (
                    node.start_position().row + 1,
                    node.end_position().row + 1
                ),
                children: vec![], // TODO: Parse struct members
            }))
        } else {
            Ok(None)
        }
    }

    /// Parse a C# enum declaration
    fn parse_csharp_enum(&self, node: Node, source: &str) -> Result<Option<ParsedModule>> {
        let mut name = None;
        let mut visibility = "internal".to_string();
        let mut docs = None;

        if let Some(name_node) = node.child_by_field_name("name") {
            name = Some(self.node_text(name_node, source));
        }

        if let Some(modifiers_node) = node.child_by_field_name("modifiers") {
            visibility = self.parse_csharp_visibility(modifiers_node, source);
        }

        docs = self.extract_docs_before_node(node, source);

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
                children: vec![], // TODO: Parse enum values
            }))
        } else {
            Ok(None)
        }
    }

    /// Parse a C# method declaration
    fn parse_csharp_method(&self, node: Node, source: &str) -> Result<Option<ParsedModule>> {
        let mut name = None;
        let mut visibility = "private".to_string(); // C# default for methods
        let mut docs = None;

        if let Some(name_node) = node.child_by_field_name("name") {
            name = Some(self.node_text(name_node, source));
        }

        if let Some(modifiers_node) = node.child_by_field_name("modifiers") {
            visibility = self.parse_csharp_visibility(modifiers_node, source);
        }

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

    /// Parse a C# namespace declaration
    fn parse_csharp_namespace(&self, node: Node, source: &str) -> Result<Option<ParsedModule>> {
        let mut name = None;
        let visibility = "public".to_string(); // Namespaces are public
        let mut docs = None;

        if let Some(name_node) = node.child_by_field_name("name") {
            name = Some(self.node_text(name_node, source));
        }

        docs = self.extract_docs_before_node(node, source);

        if let Some(namespace_name) = name {
            Ok(Some(ParsedModule {
                name: namespace_name,
                item_type: "namespace".to_string(),
                visibility,
                docs,
                signature: Some(format!("namespace {}", self.node_text(node.child_by_field_name("name").unwrap(), source))),
                line_range: (
                    node.start_position().row + 1,
                    node.end_position().row + 1
                ),
                children: vec![], // TODO: Parse namespace contents
            }))
        } else {
            Ok(None)
        }
    }

    /// Parse C# visibility modifiers
    fn parse_csharp_visibility(&self, node: Node, source: &str) -> String {
        let modifiers_text = self.node_text(node, source);
        if modifiers_text.contains("public") {
            "public".to_string()
        } else if modifiers_text.contains("private") {
            "private".to_string()
        } else if modifiers_text.contains("protected") {
            "protected".to_string()
        } else if modifiers_text.contains("internal") {
            "internal".to_string()
        } else {
            "internal".to_string() // C# default visibility
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

    /// Extract method signature (more precise than class signature)
    fn extract_method_signature(&self, node: Node, source: &str) -> String {
        let full_text = self.node_text(node, source);
        if let Some(brace_pos) = full_text.find('{') {
            full_text[..brace_pos].trim().to_string()
        } else if let Some(semicolon_pos) = full_text.find(';') {
            // Abstract method or interface method
            full_text[..semicolon_pos].trim().to_string()
        } else {
            full_text.lines().next().unwrap_or("").trim().to_string()
        }
    }

    /// Extract XML documentation comments before a node
    fn extract_docs_before_node(&self, node: Node, source: &str) -> Option<String> {
        let start_row = node.start_position().row;
        let lines: Vec<&str> = source.lines().collect();
        let mut doc_lines = Vec::new();

        // Look backwards from the node's line for XML doc comments
        for i in (0..start_row).rev() {
            if i >= lines.len() {
                continue;
            }

            let line = lines[i].trim();

            if line.starts_with("///") {
                let content = line.trim_start_matches("///").trim();
                if !content.is_empty() {
                    doc_lines.insert(0, content.to_string());
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
            } else if line.starts_with("[") {
                // Attribute, continue looking
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