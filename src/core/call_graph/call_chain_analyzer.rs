// src/core/call_graph/call_chain_analyzer.rs
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use serde::{Serialize, Deserialize};

use crate::error::Result;
use super::{CallChainGroup, MethodSignature};
use super::super::{LlmDocumenter, EnhancementRequest, EnhancementType, DocumentationContext, ParsedFile, ProjectInfo};

/// Analyzes call chain groups using LLM and maintains visited sets for efficiency
pub struct CallChainAnalyzer {
    /// Tracks which source files have been sent to LLM
    visited_files: VisitedSet,
    /// Cache of previous group analyses
    analysis_cache: HashMap<String, GroupAnalysis>,
    /// Maximum context size per LLM call
    max_context_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupAnalysis {
    /// Group being analyzed
    pub group_id: String,
    /// LLM-generated description of what these call chains accomplish
    pub description: String,
    /// Purpose of each entry point
    pub entry_point_descriptions: HashMap<MethodSignature, String>,
    /// Component interactions discovered
    pub component_interactions: Vec<ComponentInteraction>,
    /// Domain-specific insights
    pub domain_insights: Vec<DomainInsight>,
    /// Potential issues or gotchas
    pub gotchas: Vec<Gotcha>,
    /// Confidence score for this analysis
    pub confidence: f32,
    /// Whether this analysis used incremental context
    pub is_incremental: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentInteraction {
    pub from_component: String,
    pub to_component: String,
    pub interaction_type: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainInsight {
    pub category: String,
    pub insight: String,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gotcha {
    pub category: String,
    pub description: String,
    pub severity: GotchaSeverity,
    pub suggested_action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GotchaSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSynthesis {
    pub overall_description: String,
    pub key_themes: Vec<String>,
    pub critical_gotchas: Vec<Gotcha>,
    pub total_groups_analyzed: usize,
    pub overall_confidence: f32,
}

/// Tracks which source files have been sent to LLM to avoid redundant analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisitedSet {
    /// Files that have been fully analyzed
    fully_analyzed_files: HashSet<PathBuf>,
    /// Files that have been partially analyzed (summaries available)
    partially_analyzed_files: HashMap<PathBuf, String>,
    /// Cache of file summaries to use in incremental analysis
    file_summaries: HashMap<PathBuf, String>,
}

impl CallChainAnalyzer {
    pub fn new(max_context_size: usize) -> Self {
        Self {
            visited_files: VisitedSet::new(),
            analysis_cache: HashMap::new(),
            max_context_size,
        }
    }

    /// Analyze a group of call chains using LLM
    pub async fn analyze_group(
        &mut self,
        group: &CallChainGroup,
        all_files: &[ParsedFile],
        llm_documenter: &dyn LlmDocumenter,
    ) -> Result<GroupAnalysis> {
        // Check cache first
        if let Some(cached) = self.analysis_cache.get(&group.group_id) {
            return Ok(cached.clone());
        }

        // Simple analysis for now
        let analysis = GroupAnalysis {
            group_id: group.group_id.clone(),
            description: format!("Call chain group with {} chains involving {} files",
                                 group.call_chains.len(), group.involved_files.len()),
            entry_point_descriptions: HashMap::new(),
            component_interactions: Vec::new(),
            domain_insights: Vec::new(),
            gotchas: Vec::new(),
            confidence: 0.7,
            is_incremental: false,
        };

        // Cache the result
        self.analysis_cache.insert(group.group_id.clone(), analysis.clone());

        Ok(analysis)
    }

    /// Generate overall system synthesis from all group analyses
    pub fn synthesize_system_understanding(&self, analyses: &[GroupAnalysis]) -> Result<SystemSynthesis> {
        let overall_description = if analyses.is_empty() {
            "System analyzed using call-chain analysis".to_string()
        } else {
            format!("System with {} call chain groups analyzed", analyses.len())
        };

        Ok(SystemSynthesis {
            overall_description,
            key_themes: vec!["Call Chain Analysis".to_string()],
            critical_gotchas: Vec::new(),
            total_groups_analyzed: analyses.len(),
            overall_confidence: 0.7,
        })
    }
}

impl VisitedSet {
    pub fn new() -> Self {
        Self {
            fully_analyzed_files: HashSet::new(),
            partially_analyzed_files: HashMap::new(),
            file_summaries: HashMap::new(),
        }
    }
}

impl Default for CallChainAnalyzer {
    fn default() -> Self {
        Self::new(1000000)
    }
}