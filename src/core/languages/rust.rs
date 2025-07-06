use std::path::Path;
use tree_sitter::{Language, Parser, Node};

use crate::error::{CodesworthError, Result};
use super::{LanguageParser, ParsedModule};

/// Rust-specific parser using Tree-sitter
pub struct RustParser {
    parser: Parser,
}

impl RustParser {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        let rust_language = tree_sitter_rust::language();
        parser.set_language(&rust_language)
            .map_err(|e| CodesworthError::Parser(format!("Failed to set Rust language: {}", e)))?;

        Ok(Self { parser })
    }
}

impl LanguageParser for RustParser {
    fn parse(&mut self, content: &str, _file_path: &Path) -> Result<Vec<ParsedModule>> {
        let tree = self.parser.parse(content, None)
            .ok_or_else(|| CodesworthError::Parser("Failed to parse Rust code".to_string()))?;

        let root_node = tree.root_node();
        let mut modules = Vec::new();

        // Walk the AST and extract items
        self.extract_rust_items(root_node, content, &mut modules)?;

        if modules.is_empty() {
            Ok(vec![self.create_placeholder_module("module", "module")])
        } else {
            Ok(modules)
        }
    }

    fn extract_file_docs(&self, content: &str) -> Option<String> {
        let lines: Vec<&str> = content.lines().collect();
        let mut doc_lines = Vec::new();

        for line in lines {
            let trimmed = line.trim();
            if trimmed.starts_with("//!") || trimmed.starts_with("///") {
                doc_lines.push(trimmed.trim_start_matches("//!").trim_start_matches("///").trim());
            } else if trimmed.starts_with("/*") || trimmed.starts_with("*") {
                // Handle block comments
                doc_lines.push(trimmed.trim_start_matches("/*").trim_start_matches("*").trim());
            } else if !trimmed.is_empty() && !trimmed.starts_with("//") {
                // Stop at first non-comment, non-empty line
                break;
            }
        }

        if doc_lines.is_empty() {
            None
        } else {
            Some(doc_lines.join("\n"))
        }
    }

    fn file_extensions(&self) -> &[&str] {
        &["rs"]
    }

    fn language_name(&self) -> &str {
        "rust"
    }
}

impl RustParser {
    /// Extract Rust language items from AST nodes
    fn extract_rust_items(&self, node: Node, source: &str, modules: &mut Vec<ParsedModule>) -> Result<()> {
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "struct_item" => {
                    if let Some(parsed_struct) = self.parse_rust_struct(child, source)? {
                        modules.push(parsed_struct);
                    }
                }
                "function_item" => {
                    if let Some(parsed_fn) = self.parse_rust_function(child, source)? {
                        modules.push(parsed_fn);
                    }
                }
                "enum_item" => {
                    if let Some(parsed_enum) = self.parse_rust_enum(child, source)? {
                        modules.push(parsed_enum);
                    }
                }
                "impl_item" => {
                    if let Some(parsed_impl) = self.parse_rust_impl(child, source)? {
                        modules.push(parsed_impl);
                    }
                }
                "mod_item" => {
                    if let Some(parsed_mod) = self.parse_rust_mod(child, source)? {
                        modules.push(parsed_mod);
                    }
                }
                "trait_item" => {
                    if let Some(parsed_trait) = self.parse_rust_trait(child, source)? {
                        modules.push(parsed_trait);
                    }
                }
                _ => {
                    // Recursively check child nodes
                    self.extract_rust_items(child, source, modules)?;
                }
            }
        }

        Ok(())
    }

    /// Parse a Rust struct definition
    fn parse_rust_struct(&self, node: Node, source: &str) -> Result<Option<ParsedModule>> {
        let mut name = None;
        let mut visibility = "private".to_string();
        let mut docs = None;

        // Extract struct name
        if let Some(name_node) = node.child_by_field_name("name") {
            name = Some(self.node_text(name_node, source));
        }

        // Check for visibility modifier
        if let Some(vis_node) = node.child_by_field_name("visibility") {
            visibility = self.parse_visibility(vis_node, source);
        }

        // Look for documentation comments
        docs = self.extract_docs_before_node(node, source);

        // Get the full signature
        let signature = self.node_text(node, source);

        if let Some(struct_name) = name {
            Ok(Some(ParsedModule {
                name: struct_name,
                item_type: "struct".to_string(),
                visibility,
                docs,
                signature: Some(signature),
                line_range: (
                    node.start_position().row + 1,
                    node.end_position().row + 1
                ),
                children: vec![], // TODO: Parse struct fields
            }))
        } else {
            Ok(None)
        }
    }

    /// Parse a Rust function definition
    fn parse_rust_function(&self, node: Node, source: &str) -> Result<Option<ParsedModule>> {
        let mut name = None;
        let mut visibility = "private".to_string();
        let mut docs = None;

        // Extract function name
        if let Some(name_node) = node.child_by_field_name("name") {
            name = Some(self.node_text(name_node, source));
        }

        // Check for visibility modifier - look for pub keyword in the function
        let function_text = self.node_text(node, source);
        if function_text.trim_start().starts_with("pub") {
            visibility = "public".to_string();
        }

        // Look for documentation comments
        docs = self.extract_docs_before_node(node, source);

        // Get the function signature (everything up to the opening brace)
        let signature = self.extract_function_signature(node, source);

        if let Some(fn_name) = name {
            Ok(Some(ParsedModule {
                name: fn_name,
                item_type: "function".to_string(),
                visibility,
                docs,
                signature: Some(signature),
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

    /// Parse a Rust enum definition
    fn parse_rust_enum(&self, node: Node, source: &str) -> Result<Option<ParsedModule>> {
        let mut name = None;
        let mut visibility = "private".to_string();
        let mut docs = None;

        if let Some(name_node) = node.child_by_field_name("name") {
            name = Some(self.node_text(name_node, source));
        }

        if let Some(vis_node) = node.child_by_field_name("visibility") {
            visibility = self.parse_visibility(vis_node, source);
        }

        docs = self.extract_docs_before_node(node, source);
        let signature = self.node_text(node, source);

        if let Some(enum_name) = name {
            Ok(Some(ParsedModule {
                name: enum_name,
                item_type: "enum".to_string(),
                visibility,
                docs,
                signature: Some(signature),
                line_range: (
                    node.start_position().row + 1,
                    node.end_position().row + 1
                ),
                children: vec![], // TODO: Parse enum variants
            }))
        } else {
            Ok(None)
        }
    }

    /// Parse a Rust impl block
    fn parse_rust_impl(&self, node: Node, source: &str) -> Result<Option<ParsedModule>> {
        let mut type_name = "Unknown".to_string();
        let mut docs = None;

        // Try to get the type being implemented
        if let Some(type_node) = node.child_by_field_name("type") {
            type_name = self.node_text(type_node, source);
        }

        docs = self.extract_docs_before_node(node, source);

        // Extract methods from impl block - need to look for declaration_list
        let mut methods = Vec::new();
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if child.kind() == "declaration_list" {
                // Look inside the declaration list for function items
                let mut inner_cursor = child.walk();
                for inner_child in child.children(&mut inner_cursor) {
                    if inner_child.kind() == "function_item" {
                        if let Some(method) = self.parse_rust_function(inner_child, source)? {
                            methods.push(method);
                        }
                    }
                }
            }
        }

        // Create a concise impl signature
        let impl_signature = format!("impl {}", type_name);

        Ok(Some(ParsedModule {
            name: format!("impl {}", type_name),
            item_type: "impl".to_string(),
            visibility: "public".to_string(),
            docs,
            signature: Some(impl_signature),
            line_range: (
                node.start_position().row + 1,
                node.end_position().row + 1
            ),
            children: methods,
        }))
    }

    /// Parse a Rust module definition
    fn parse_rust_mod(&self, node: Node, source: &str) -> Result<Option<ParsedModule>> {
        let mut name = None;
        let mut visibility = "private".to_string();
        let mut docs = None;

        if let Some(name_node) = node.child_by_field_name("name") {
            name = Some(self.node_text(name_node, source));
        }

        if let Some(vis_node) = node.child_by_field_name("visibility") {
            visibility = self.parse_visibility(vis_node, source);
        }

        docs = self.extract_docs_before_node(node, source);

        if let Some(mod_name) = name {
            Ok(Some(ParsedModule {
                name: mod_name,
                item_type: "module".to_string(),
                visibility,
                docs,
                signature: Some(self.node_text(node, source)),
                line_range: (
                    node.start_position().row + 1,
                    node.end_position().row + 1
                ),
                children: vec![], // TODO: Parse module contents
            }))
        } else {
            Ok(None)
        }
    }

    /// Parse a Rust trait definition
    fn parse_rust_trait(&self, node: Node, source: &str) -> Result<Option<ParsedModule>> {
        let mut name = None;
        let mut visibility = "private".to_string();
        let mut docs = None;

        if let Some(name_node) = node.child_by_field_name("name") {
            name = Some(self.node_text(name_node, source));
        }

        if let Some(vis_node) = node.child_by_field_name("visibility") {
            visibility = self.parse_visibility(vis_node, source);
        }

        docs = self.extract_docs_before_node(node, source);

        if let Some(trait_name) = name {
            Ok(Some(ParsedModule {
                name: trait_name,
                item_type: "trait".to_string(),
                visibility,
                docs,
                signature: Some(self.node_text(node, source)),
                line_range: (
                    node.start_position().row + 1,
                    node.end_position().row + 1
                ),
                children: vec![], // TODO: Parse trait methods
            }))
        } else {
            Ok(None)
        }
    }

    /// Parse visibility modifiers
    fn parse_visibility(&self, node: Node, source: &str) -> String {
        let vis_text = self.node_text(node, source);
        if vis_text.starts_with("pub") {
            "public".to_string()
        } else {
            "private".to_string()
        }
    }

    /// Extract text content of a node
    fn node_text(&self, node: Node, source: &str) -> String {
        source[node.byte_range()].to_string()
    }

    /// Extract function signature (without body)
    fn extract_function_signature(&self, node: Node, source: &str) -> String {
        // Get the full function text and find where the body starts
        let full_text = self.node_text(node, source);

        // Find the opening brace that starts the function body
        let lines: Vec<&str> = full_text.lines().collect();
        let mut signature_lines = Vec::new();

        for line in lines {
            let trimmed = line.trim();
            if trimmed.contains('{') && !trimmed.starts_with("//") {
                // This line contains the opening brace
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

    /// Extract documentation comments before a node
    fn extract_docs_before_node(&self, node: Node, source: &str) -> Option<String> {
        let start_row = node.start_position().row;
        let lines: Vec<&str> = source.lines().collect();
        let mut doc_lines = Vec::new();

        // Look backwards from the node's line for doc comments
        for i in (0..start_row).rev() {
            if i >= lines.len() {
                continue;
            }

            let line = lines[i].trim();
            if line.starts_with("///") {
                let doc = line.trim_start_matches("///").trim();
                doc_lines.insert(0, doc.to_string());
            } else if line.starts_with("//!") {
                let doc = line.trim_start_matches("//!").trim();
                doc_lines.insert(0, doc.to_string());
            } else if line.is_empty() {
                // Allow empty lines in doc blocks
                continue;
            } else if line.starts_with("//") {
                // Regular comment, stop looking
                break;
            } else if line.starts_with("#[") {
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