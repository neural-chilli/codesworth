//! Language-specific parsers for different programming languages
//!
//! Each language gets its own module with a consistent interface for parsing
//! source code into structured documentation data.

mod rust;
mod java;
mod python;
mod csharp;
mod javascript;

pub use rust::RustParser;
pub use java::JavaParser;
pub use python::PythonParser;
pub use csharp::CSharpParser;
pub use javascript::JavaScriptParser;

use crate::error::Result;
use super::{ParsedModule, ParsedFile};

/// Trait that all language parsers must implement
pub trait LanguageParser {
    /// Parse source code and extract structured information
    fn parse(&mut self, content: &str, file_path: &std::path::Path) -> Result<Vec<ParsedModule>>;

    /// Extract file-level documentation from source code
    fn extract_file_docs(&self, content: &str) -> Option<String>;

    /// Get the file extensions this parser handles
    fn file_extensions(&self) -> &[&str];

    /// Get the language name
    fn language_name(&self) -> &str;
}