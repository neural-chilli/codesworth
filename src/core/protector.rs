use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::error::{CodesworthError, Result};

/// Represents a protected region in documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectedRegion {
    /// Unique identifier for the protected region
    pub id: String,

    /// Human-readable label
    pub label: Option<String>,

    /// The protected content
    pub content: String,

    /// Line range in the document
    pub line_range: (usize, usize),
}

/// Handles protection and merging of human edits in documentation
pub struct EditProtector {
    /// Regex for detecting protected region start
    protected_start_regex: Regex,

    /// Regex for detecting protected region end
    protected_end_regex: Regex,
}

impl EditProtector {
    pub fn new() -> Self {
        Self {
            protected_start_regex: Regex::new(r"<!--\s*PROTECTED(?::\s*(.+?))?\s*-->")
                .expect("Invalid protected start regex"),
            protected_end_regex: Regex::new(r"<!--\s*/PROTECTED\s*-->")
                .expect("Invalid protected end regex"),
        }
    }

    /// Extract all protected regions from existing content
    pub fn extract_protected_regions(&self, content: &str) -> Result<Vec<ProtectedRegion>> {
        let mut regions = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            if let Some(start_match) = self.protected_start_regex.find(lines[i]) {
                // Extract label if present
                let label = self.protected_start_regex
                    .captures(lines[i])
                    .and_then(|caps| caps.get(1))
                    .map(|m| m.as_str().trim().to_string());

                // Find the end of the protected region
                let start_line = i + 1;
                let mut end_line = start_line;
                let mut found_end = false;

                for j in (i + 1)..lines.len() {
                    if self.protected_end_regex.is_match(lines[j]) {
                        end_line = j;
                        found_end = true;
                        break;
                    }
                }

                if !found_end {
                    return Err(CodesworthError::ProtectedRegion(
                        format!("Unclosed protected region starting at line {}", i + 1)
                    ));
                }

                // Extract the content between markers
                let protected_content = lines[start_line..end_line].join("\n");

                // Generate ID from label or content
                let id = label.clone().unwrap_or_else(|| {
                    self.generate_region_id(&protected_content)
                });

                regions.push(ProtectedRegion {
                    id,
                    label,
                    content: protected_content,
                    line_range: (start_line + 1, end_line + 1), // 1-indexed
                });

                i = end_line + 1;
            } else {
                i += 1;
            }
        }

        Ok(regions)
    }

    /// Merge new generated content with existing protected regions
    pub fn merge_with_existing(&self, new_content: &str, existing_content: &str) -> Result<String> {
        // Extract protected regions from existing content
        let protected_regions = self.extract_protected_regions(existing_content)?;

        if protected_regions.is_empty() {
            // No protected regions, return new content as-is
            return Ok(new_content.to_string());
        }

        // Strategy: Replace any protected regions in new content with the preserved versions
        let mut result = new_content.to_string();

        for region in &protected_regions {
            // Look for the same protected region in new content (by label or ID)
            let search_pattern = if let Some(ref label) = region.label {
                format!("<!-- PROTECTED: {} -->", label)
            } else {
                "<!-- PROTECTED -->".to_string()
            };

            if let Some(start_pos) = result.find(&search_pattern) {
                // Find the end marker
                if let Some(end_pos) = result[start_pos..].find("<!-- /PROTECTED -->") {
                    let actual_end_pos = start_pos + end_pos + "<!-- /PROTECTED -->".len();

                    // Replace the entire section with the preserved version
                    let preserved_block = self.format_protected_region(region);
                    result.replace_range(start_pos..actual_end_pos, &preserved_block);
                }
            } else {
                // Protected region doesn't exist in new template, append it at the end
                result.push_str("\n\n");
                result.push_str(&self.format_protected_region(region));
            }
        }

        Ok(result)
    }

    /// Insert protected region markers around content
    pub fn protect_content(&self, content: &str, label: Option<&str>) -> String {
        let start_marker = if let Some(label) = label {
            format!("<!-- PROTECTED: {} -->", label)
        } else {
            "<!-- PROTECTED -->".to_string()
        };

        format!("{}\n{}\n<!-- /PROTECTED -->", start_marker, content)
    }

    /// Check if content contains any protected regions
    pub fn has_protected_regions(&self, content: &str) -> bool {
        self.protected_start_regex.is_match(content)
    }

    /// Validate that all protected regions are properly closed
    pub fn validate_protected_regions(&self, content: &str) -> Result<()> {
        let _ = self.extract_protected_regions(content)?;
        Ok(())
    }

    // Private helper methods

    /// Generate a unique ID for a protected region based on its content
    fn generate_region_id(&self, content: &str) -> String {
        use sha2::{Sha256, Digest};

        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let hash = format!("{:x}", hasher.finalize());

        // Use first 8 characters of hash for readability
        format!("region-{}", &hash[..8])
    }

    /// Format a protected region with its markers
    fn format_protected_region(&self, region: &ProtectedRegion) -> String {
        let start_marker = if let Some(ref label) = region.label {
            format!("<!-- PROTECTED: {} -->", label)
        } else {
            "<!-- PROTECTED -->".to_string()
        };

        format!("{}\n{}\n<!-- /PROTECTED -->", start_marker, region.content)
    }

    /// Find a good insertion point for a protected region in new content
    fn find_insertion_point(&self, content: &str, region: &ProtectedRegion) -> Option<usize> {
        // Strategy: Look for section headers or similar structural elements
        // that might indicate where this protected content should go

        // For now, implement a simple approach: insert before first major heading
        // if the region looks like it contains architectural decisions or explanations

        if region.label.as_ref().map_or(false, |label| {
            label.to_lowercase().contains("architecture") ||
                label.to_lowercase().contains("decision") ||
                label.to_lowercase().contains("overview")
        }) {
            // Try to insert after the main title but before first section
            let lines: Vec<&str> = content.lines().collect();
            for (i, line) in lines.iter().enumerate() {
                if line.starts_with("## ") {
                    // Found first section, insert before it
                    return Some(lines[..i].join("\n").len());
                }
            }
        }

        None
    }

    /// Insert a protected region at a specific position in the content
    fn insert_protected_region_at(&self, content: &str, region: &ProtectedRegion, position: usize) -> Result<String> {
        if position > content.len() {
            return Err(CodesworthError::ProtectedRegion(
                "Invalid insertion position".to_string()
            ));
        }

        let protected_block = self.format_protected_region(region);
        let mut result = String::new();
        result.push_str(&content[..position]);
        result.push_str("\n\n");
        result.push_str(&protected_block);
        result.push_str("\n\n");
        result.push_str(&content[position..]);

        Ok(result)
    }
}

impl Default for EditProtector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_protected_regions() {
        let protector = EditProtector::new();
        let content = r#"# Test Document

<!-- PROTECTED: Architecture Decision -->
This is a protected section about architecture.
It should be preserved across regenerations.
<!-- /PROTECTED -->

## Some Generated Section

This content will be regenerated.

<!-- PROTECTED -->
Another protected section without a label.
<!-- /PROTECTED -->
"#;

        let regions = protector.extract_protected_regions(content).unwrap();
        assert_eq!(regions.len(), 2);

        assert_eq!(regions[0].label, Some("Architecture Decision".to_string()));
        assert!(regions[0].content.contains("protected section about architecture"));

        assert_eq!(regions[1].label, None);
        assert!(regions[1].content.contains("Another protected section"));
    }

    #[test]
    fn test_protect_content() {
        let protector = EditProtector::new();
        let content = "This is important content that should be preserved.";
        let protected = protector.protect_content(content, Some("Important Note"));

        assert!(protected.contains("<!-- PROTECTED: Important Note -->"));
        assert!(protected.contains("<!-- /PROTECTED -->"));
        assert!(protected.contains(content));
    }

    #[test]
    fn test_has_protected_regions() {
        let protector = EditProtector::new();

        let with_protection = "<!-- PROTECTED -->\nContent\n<!-- /PROTECTED -->";
        let without_protection = "Just regular content";

        assert!(protector.has_protected_regions(with_protection));
        assert!(!protector.has_protected_regions(without_protection));
    }
}