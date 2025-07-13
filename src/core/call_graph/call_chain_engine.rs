// src/core/call_graph/call_chain_engine.rs
use std::path::{Path, PathBuf};
use tracing::{info, debug};

use crate::error::Result;
use super::{
    CallGraph, EntryPointDetector, CallChainTracer, CallChainGrouper, CallChainAnalyzer,
    CallChain, CallChainGroup, GroupAnalysis, SystemSynthesis, EntryPoint
};
use super::super::{LlmDocumenter, CodeParser, ParsedFile};

/// Main orchestrator for call-chain analysis
pub struct CallChainEngine {
    entry_point_detector: EntryPointDetector,
    call_chain_tracer: CallChainTracer,
    call_chain_grouper: CallChainGrouper,
    call_chain_analyzer: CallChainAnalyzer,
    max_depth: usize,
}

#[derive(Debug, Clone)]
pub struct CallChainAnalysisResult {
    /// The complete call graph
    pub call_graph: CallGraph,
    /// All detected entry points
    pub entry_points: Vec<EntryPoint>,
    /// All traced call chains
    pub call_chains: Vec<CallChain>,
    /// Grouped call chains
    pub call_chain_groups: Vec<CallChainGroup>,
    /// LLM analysis of each group
    pub group_analyses: Vec<GroupAnalysis>,
    /// Overall system understanding
    pub system_synthesis: SystemSynthesis,
    /// Performance statistics
    pub stats: AnalysisStatistics,
}

#[derive(Debug, Clone)]
pub struct AnalysisStatistics {
    pub total_methods: usize,
    pub total_calls: usize,
    pub entry_points_found: usize,
    pub call_chains_traced: usize,
    pub groups_created: usize,
    pub llm_calls_made: usize,
    pub files_analyzed: usize,
    pub analysis_time_ms: u128,
}

impl CallChainEngine {
    pub fn new(max_depth: usize, max_context_size: usize) -> Self {
        Self {
            entry_point_detector: EntryPointDetector::new(),
            call_chain_tracer: CallChainTracer::new(max_depth),
            call_chain_grouper: CallChainGrouper::new(),
            call_chain_analyzer: CallChainAnalyzer::new(max_context_size),
            max_depth,
        }
    }

    /// Perform complete call-chain analysis on a codebase
    pub async fn analyze_codebase<P: AsRef<Path>>(
        &mut self,
        source_dir: P,
        parser: &mut CodeParser,
        llm_documenter: Option<&dyn LlmDocumenter>,
    ) -> Result<CallChainAnalysisResult> {
        let start_time = std::time::Instant::now();
        info!("🔍 Starting call-chain analysis...");

        // Step 1: Parse all files
        info!("📖 Parsing source files...");
        let parsed_files = parser.parse_directory(&source_dir).await?;
        info!("Found {} source files", parsed_files.len());

        // Step 2: Build call graph
        info!("🕸️ Building call graph...");
        let call_graph = CallGraph::build_from_files(&parsed_files)?;
        let graph_stats = call_graph.get_statistics();
        info!("Built call graph: {} methods, {} calls, {} cycles detected",
              graph_stats.total_methods, graph_stats.total_calls, graph_stats.cycles);

        // Step 3: Detect entry points
        info!("🚪 Detecting entry points...");
        let entry_points = self.entry_point_detector.detect_entry_points(&call_graph)?;
        info!("Found {} entry points", entry_points.len());

        for ep in &entry_points {
            debug!("Entry point: {} (type: {:?}, confidence: {:.2})",
                   ep.signature.display_name(), ep.entry_type, ep.confidence);
        }

        // Step 4: Trace call chains
        info!("🔗 Tracing call chains (max depth: {})...", self.max_depth);
        let call_chains = self.call_chain_tracer.trace_all_chains(&call_graph, &entry_points)?;
        info!("Traced {} call chains", call_chains.len());

        // Step 5: Group call chains by file sets
        info!("📦 Grouping call chains by involved files...");
        let call_chain_groups = self.call_chain_grouper.group_call_chains(call_chains.clone())?;
        let grouping_stats = self.call_chain_grouper.get_grouping_statistics(&call_chain_groups);
        info!("Created {} groups (avg {:.1} chains per group)",
              grouping_stats.total_groups, grouping_stats.avg_chains_per_group);

        // Step 6: Analyze groups with LLM (if available)
        let mut group_analyses = Vec::new();
        let mut llm_calls_made = 0;

        if let Some(llm) = llm_documenter {
            info!("🧠 Analyzing groups with LLM...");

            for (i, group) in call_chain_groups.iter().enumerate() {
                let group_name = self.call_chain_grouper.get_group_name(group);
                info!("Analyzing group {}/{}: {}", i + 1, call_chain_groups.len(), group_name);

                match self.call_chain_analyzer.analyze_group(group, &parsed_files, llm).await {
                    Ok(analysis) => {
                        llm_calls_made += 1;
                        debug!("Group analysis complete (confidence: {:.2}): {}",
                               analysis.confidence, 
                               analysis.description.chars().take(100).collect::<String>());
                        group_analyses.push(analysis);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to analyze group {}: {}", group_name, e);
                        // Continue with other groups
                    }
                }
            }
        } else {
            info!("No LLM configured, skipping group analysis");
        }

        // Step 7: Synthesize overall understanding
        info!("🎯 Synthesizing system understanding...");
        let system_synthesis = if !group_analyses.is_empty() {
            self.call_chain_analyzer.synthesize_system_understanding(&group_analyses)?
        } else {
            SystemSynthesis {
                overall_description: "Call-chain analysis completed without LLM enhancement".to_string(),
                key_themes: vec!["Code Structure".to_string()],
                critical_gotchas: vec![],
                total_groups_analyzed: 0,
                overall_confidence: 0.5,
            }
        };

        let analysis_time_ms = start_time.elapsed().as_millis();
        info!("✅ Call-chain analysis complete in {}ms", analysis_time_ms);

        let stats = AnalysisStatistics {
            total_methods: graph_stats.total_methods,
            total_calls: graph_stats.total_calls,
            entry_points_found: entry_points.len(),
            call_chains_traced: call_chains.len(),
            groups_created: call_chain_groups.len(),
            llm_calls_made,
            files_analyzed: parsed_files.len(),
            analysis_time_ms,
        };

        Ok(CallChainAnalysisResult {
            call_graph,
            entry_points,
            call_chains,
            call_chain_groups,
            group_analyses,
            system_synthesis,
            stats,
        })
    }

    /// Generate documentation from call-chain analysis
    pub async fn generate_documentation(
        &self,
        analysis_result: &CallChainAnalysisResult,
        output_dir: &Path,
    ) -> Result<()> {
        info!("📝 Generating documentation from call-chain analysis...");

        // Create output directory
        std::fs::create_dir_all(output_dir)?;

        // Generate system overview
        self.generate_system_overview(analysis_result, output_dir).await?;

        // Generate group documentation
        self.generate_group_documentation(analysis_result, output_dir).await?;

        // Generate call graph visualization data
        self.generate_call_graph_data(analysis_result, output_dir).await?;

        info!("✅ Documentation generation complete");
        Ok(())
    }

    /// Generate system-level overview documentation
    async fn generate_system_overview(
        &self,
        analysis_result: &CallChainAnalysisResult,
        output_dir: &Path,
    ) -> Result<()> {
        let mut content = String::new();

        // Header with metadata
        content.push_str("# System Overview - Call Chain Analysis\n\n");
        content.push_str(&format!(
            "Generated: {}\n\
            Analysis Statistics: {} methods, {} call chains, {} groups\n\n",
            chrono::Utc::now().to_rfc3339(),
            analysis_result.stats.total_methods,
            analysis_result.stats.call_chains_traced,
            analysis_result.stats.groups_created
        ));

        // System synthesis
        let synthesis = &analysis_result.system_synthesis;
        content.push_str("## System Understanding\n\n");
        content.push_str(&synthesis.overall_description);
        content.push_str("\n\n");

        if !synthesis.key_themes.is_empty() {
            content.push_str("### Key Themes\n\n");
            for theme in &synthesis.key_themes {
                content.push_str(&format!("- {}\n", theme));
            }
            content.push_str("\n");
        }

        // Entry points
        content.push_str("## Entry Points\n\n");
        content.push_str("These are the main ways users and external systems interact with this codebase:\n\n");

        for entry_point in &analysis_result.entry_points {
            content.push_str(&format!(
                "### {} ({:?})\n\n",
                entry_point.signature.display_name(),
                entry_point.entry_type
            ));
            content.push_str(&format!("**File**: {}\n", entry_point.signature.file_path.display()));
            content.push_str(&format!("**Confidence**: {:.2}\n", entry_point.confidence));
            content.push_str(&format!("**Reasoning**: {}\n\n", entry_point.reasoning));
        }

        // Group summaries
        content.push_str("## Execution Path Groups\n\n");
        content.push_str("Related execution paths grouped by the files they involve:\n\n");

        for (i, group) in analysis_result.call_chain_groups.iter().enumerate() {
            let group_name = self.call_chain_grouper.get_group_name(group);
            content.push_str(&format!("### {}\n\n", group_name));

            content.push_str(&format!(
                "- **Chains**: {}\n\
                - **Files**: {}\n\
                - **Complexity**: {}\n",
                group.call_chains.len(),
                group.involved_files.len(),
                group.total_complexity
            ));

            if let Some(analysis) = analysis_result.group_analyses.get(i) {
                content.push_str(&format!("- **Purpose**: {}\n",
                                          analysis.description.split('.').next().unwrap_or("Unknown")));
            }

            content.push_str(&format!("\n[View detailed analysis](./groups/{}.md)\n\n", group.group_id));
        }

        // Write overview file
        let overview_path = output_dir.join("README.md");
        std::fs::write(overview_path, content)?;

        Ok(())
    }

    /// Generate detailed documentation for each group
    async fn generate_group_documentation(
        &self,
        analysis_result: &CallChainAnalysisResult,
        output_dir: &Path,
    ) -> Result<()> {
        let groups_dir = output_dir.join("groups");
        std::fs::create_dir_all(&groups_dir)?;

        for (group, analysis) in analysis_result.call_chain_groups.iter()
            .zip(analysis_result.group_analyses.iter()) {

            let mut content = String::new();
            let group_name = self.call_chain_grouper.get_group_name(group);

            // Header
            content.push_str(&format!("# {}\n\n", group_name));
            content.push_str(&format!("**Group ID**: {}\n", group.group_id));
            content.push_str(&format!("**Analysis Confidence**: {:.2}\n\n", analysis.confidence));

            // Description
            content.push_str("## What This Code Does\n\n");
            content.push_str(&analysis.description);
            content.push_str("\n\n");

            // Execution paths
            content.push_str("## Execution Paths\n\n");
            for (i, chain) in group.call_chains.iter().enumerate() {
                content.push_str(&format!(
                    "### Path {}: {} (confidence: {:.2})\n\n",
                    i + 1,
                    chain.entry_point.signature.display_name(),
                    chain.entry_point.confidence
                ));

                for step in &chain.steps {
                    let indent = "  ".repeat(step.depth);
                    content.push_str(&format!(
                        "{}{}. {} ({}:{})\n",
                        indent,
                        step.depth,
                        step.method.display_name(),
                        step.method.file_path.file_name()
                            .unwrap_or_default()
                            .to_string_lossy(),
                        step.call_site_line
                    ));
                }
                content.push_str("\n");
            }

            // Involved files
            content.push_str("## Files Involved\n\n");
            for file in &group.involved_files {
                content.push_str(&format!("- {}\n", file.display()));
            }
            content.push_str("\n");

            // Write group file
            let group_file = groups_dir.join(format!("{}.md", group.group_id));
            std::fs::write(group_file, content)?;
        }

        Ok(())
    }

    /// Generate call graph data for visualization
    async fn generate_call_graph_data(
        &self,
        analysis_result: &CallChainAnalysisResult,
        output_dir: &Path,
    ) -> Result<()> {
        // Generate JSON data for potential visualization tools
        let graph_data = serde_json::json!({
            "nodes": analysis_result.call_graph.nodes.values().collect::<Vec<_>>(),
            "edges": analysis_result.call_graph.edges,
            "entry_points": analysis_result.entry_points,
            "statistics": analysis_result.call_graph.get_statistics()
        });

        let graph_file = output_dir.join("call_graph.json");
        std::fs::write(graph_file, serde_json::to_string_pretty(&graph_data)?)?;

        Ok(())
    }
}

impl Default for CallChainEngine {
    fn default() -> Self {
        Self::new(6, 1000000) // 6 levels deep, 1M token context
    }
}