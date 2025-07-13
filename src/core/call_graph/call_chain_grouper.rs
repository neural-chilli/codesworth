// src/core/call_graph/call_chain_grouper.rs
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use serde::{Serialize, Deserialize};

use crate::error::Result;
use super::{CallChain, MethodSignature};

/// Groups call chains that involve the exact same set of source files
pub struct CallChainGrouper;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallChainGroup {
    /// Unique identifier for this group
    pub group_id: String,
    /// All call chains in this group
    pub call_chains: Vec<CallChain>,
    /// The exact set of files involved in all chains
    pub involved_files: HashSet<PathBuf>,
    /// Combined complexity score for the group
    pub total_complexity: u32,
    /// Primary entry points (highest confidence)
    pub primary_entry_points: Vec<MethodSignature>,
    /// All unique methods involved across all chains
    pub all_methods: HashSet<MethodSignature>,
}

impl CallChainGrouper {
    pub fn new() -> Self {
        Self
    }

    /// Group call chains by their exact file sets
    pub fn group_call_chains(&self, call_chains: Vec<CallChain>) -> Result<Vec<CallChainGroup>> {
        let mut file_set_to_chains: HashMap<HashSet<PathBuf>, Vec<CallChain>> = HashMap::new();

        // Group chains by exact file sets
        for chain in call_chains {
            let file_set = chain.involved_files.clone();
            file_set_to_chains.entry(file_set).or_insert_with(Vec::new).push(chain);
        }

        // Convert groups to structured format
        let mut groups = Vec::new();
        for (file_set, chains) in file_set_to_chains {
            let group = self.create_group(file_set, chains)?;
            groups.push(group);
        }

        // Sort groups by total complexity (most complex first)
        groups.sort_by(|a, b| b.total_complexity.cmp(&a.total_complexity));

        Ok(groups)
    }

    /// Create a structured group from file set and chains
    fn create_group(&self, file_set: HashSet<PathBuf>, chains: Vec<CallChain>) -> Result<CallChainGroup> {
        let group_id = self.generate_group_id(&file_set);

        let total_complexity: u32 = chains.iter()
            .map(|chain| chain.complexity_score)
            .sum();

        // Find primary entry points (highest confidence)
        let mut primary_entry_points = Vec::new();
        let mut max_confidence = 0.0;

        for chain in &chains {
            if chain.entry_point.confidence > max_confidence {
                max_confidence = chain.entry_point.confidence;
                primary_entry_points.clear();
                primary_entry_points.push(chain.entry_point.signature.clone());
            } else if (chain.entry_point.confidence - max_confidence).abs() < 0.01 {
                primary_entry_points.push(chain.entry_point.signature.clone());
            }
        }

        // Collect all unique methods
        let mut all_methods = HashSet::new();
        for chain in &chains {
            for step in &chain.steps {
                all_methods.insert(step.method.clone());
            }
        }

        Ok(CallChainGroup {
            group_id,
            call_chains: chains,
            involved_files: file_set,
            total_complexity,
            primary_entry_points,
            all_methods,
        })
    }

    /// Generate a unique ID for the group based on file set
    fn generate_group_id(&self, file_set: &HashSet<PathBuf>) -> String {
        use sha2::{Sha256, Digest};

        // Sort files for consistent hashing
        let mut sorted_files: Vec<_> = file_set.iter().collect();
        sorted_files.sort();

        let mut hasher = Sha256::new();
        for file in sorted_files {
            hasher.update(file.to_string_lossy().as_bytes());
        }

        let hash = format!("{:x}", hasher.finalize());
        format!("group-{}", &hash[..8])
    }

    /// Get a human-readable name for the group
    pub fn get_group_name(&self, group: &CallChainGroup) -> String {
        // Try to use primary entry point names
        if !group.primary_entry_points.is_empty() {
            let entry_names: Vec<String> = group.primary_entry_points.iter()
                .map(|ep| ep.display_name())
                .collect();
            return format!("Group: {}", entry_names.join(", "));
        }

        format!("Group: {} files", group.involved_files.len())
    }

    /// Get statistics for all groups
    pub fn get_grouping_statistics(&self, groups: &[CallChainGroup]) -> GroupingStats {
        let total_chains: usize = groups.iter().map(|g| g.call_chains.len()).sum();
        let total_files: usize = groups.iter()
            .flat_map(|g| &g.involved_files)
            .collect::<HashSet<_>>()
            .len();

        let largest_group_size = groups.iter()
            .map(|g| g.call_chains.len())
            .max()
            .unwrap_or(0);

        let avg_chains_per_group = if groups.is_empty() {
            0.0
        } else {
            total_chains as f64 / groups.len() as f64
        };

        GroupingStats {
            total_groups: groups.len(),
            total_chains,
            total_files,
            largest_group_size,
            avg_chains_per_group,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupingStats {
    pub total_groups: usize,
    pub total_chains: usize,
    pub total_files: usize,
    pub largest_group_size: usize,
    pub avg_chains_per_group: f64,
}

impl Default for CallChainGrouper {
    fn default() -> Self {
        Self::new()
    }
}
