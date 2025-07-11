# Codesworth Next Steps

## Overview

This document outlines the key architectural changes needed to make Codesworth a practical, high-quality documentation generator. The current implementation has good foundations but needs restructuring to be useful for real codebases.

## Core Problems to Solve

### 1. **Documentation Granularity**
- **Problem**: Current approach generates per-file docs, creating information overload
- **Solution**: Move to package/module-level documentation with intelligent hierarchy
- **Impact**: More navigable, useful documentation that mirrors developer mental models

### 2. **LLM Usage Efficiency**
- **Problem**: Too many individual LLM calls make the tool slow and expensive
- **Solution**: Batch package analysis into consolidated, context-rich prompts
- **Impact**: Faster execution, better coherence, more cost-effective

### 3. **Context Integration**
- **Problem**: Missing human-authored architectural context
- **Solution**: Scan and integrate existing docs, ADRs, READMEs into LLM context
- **Impact**: AI-generated docs that reflect actual system understanding

### 4. **Navigation and Discoverability**
- **Problem**: No clear path through the documentation
- **Solution**: Generate hierarchical indexes and cross-references using markdown
- **Impact**: Documentation people actually use instead of ignore

### 5. **Prompting Quality**
- **Problem**: Over-prescriptive prompts create tunnel vision
- **Solution**: Balanced prompts that give overview + highlight important details
- **Impact**: More useful, less generic documentation

## Architectural Changes Required

### Phase 1: Restructure Documentation Hierarchy

#### 1.1 New Documentation Model
```rust
pub struct DocumentationHierarchy {
    pub system_overview: SystemDoc,        // Single app-level overview
    pub domain_docs: Vec<DomainDoc>,       // Major functional areas
    pub package_docs: Vec<PackageDoc>,     // Module/package level (main focus)
    pub critical_components: Vec<ComponentDoc>, // Only for complex public APIs
}

pub struct PackageDoc {
    pub overview: String,                  // What this package does
    pub key_details: String,              // Gotchas, patterns, dependencies
    pub public_api: Vec<ApiDoc>,          // Exported functions/types
    pub cross_references: Vec<String>,     // Links to related packages
}
```

#### 1.2 Package Analysis Engine
```rust
pub struct PackageAnalysis {
    pub files: Vec<ParsedFile>,
    pub public_api: ApiSurface,
    pub dependencies: DependencyAnalysis,
    pub complexity_indicators: ComplexityMetrics,
    pub architectural_significance: f32,
}

pub struct DependencyAnalysis {
    pub external_deps: Vec<ExternalDependency>,
    pub internal_deps: Vec<InternalDependency>,
    pub unusual_imports: Vec<UnusualImport>,
}
```

### Phase 2: Batch LLM Processing

#### 2.1 Consolidated Context Building
```rust
pub struct BatchDocumentationRequest {
    pub package_analysis: PackageAnalysis,
    pub human_context: HumanContext,
    pub system_context: SystemContext,
    pub enhancement_focus: AnalysisFocus,
}

pub struct HumanContext {
    pub readme_content: Option<String>,
    pub architecture_docs: Vec<ArchitectureDoc>,
    pub adrs: Vec<ArchitecturalDecision>,
    pub inline_comments: Vec<ArchitecturalComment>,
}
```

#### 2.2 Improved Prompting Strategy
- **Balanced approach**: Overview + key details
- **Open-ended discovery**: Avoid leading questions
- **Context-rich**: Include human docs without interpretation
- **Practical focus**: Emphasize maintainability insights

### Phase 3: Enhanced Context Detection

#### 3.1 Human Documentation Scanner
```rust
pub struct ContextScanner {
    // Scan standard locations for human docs
    // Extract architectural insights from comments
    // Identify cross-cutting concerns
    // Build dependency graphs
}
```

#### 3.2 Gotcha Detection
- Concurrency patterns that could deadlock
- Performance assumptions
- Configuration dependencies
- Error handling quirks
- State management complexities

### Phase 4: Navigation and Output

#### 4.1 Markdown-Based Navigation
- Multi-level README.md structure
- Table of contents with relative links
- Cross-reference system
- Metadata-driven organization

#### 4.2 Progressive Enhancement
- Basic structural docs first (fast)
- LLM enhancement second (slower)
- Configurable enhancement levels

## Implementation Priorities

### Start with...
1. **Refactor to package-level analysis**
    - Modify `CodeParser` to group files by package/module
    - Update `DocGenerator` to work with `PackageAnalysis`
    - Implement basic package documentation template

2. **Improve prompting**
    - Replace prescriptive prompts with balanced overview + details approach
    - Test prompt variations for quality and consistency
    - Add context integration without bias

### ...then...
3. **Batch LLM processing**
    - Consolidate package analysis into single LLM calls
    - Implement intelligent context window management
    - Add caching to avoid regenerating unchanged packages

4. **Human context integration**
    - Scan for README, architecture docs, ADRs
    - Extract architectural comments from code
    - Build system-level context for LLM

### ...and then...
5. **Navigation system**
    - Generate hierarchical index structure
    - Implement cross-reference linking
    - Create system-level overview document

6. **Gotcha detection**
    - Identify language-specific patterns to highlight
    - Detect unusual dependencies and imports
    - Surface performance and concurrency concerns

### ...finally
7. **Quality refinement**
    - A/B test different prompting strategies
    - Gather user feedback on documentation utility
    - Iterate on output format and structure

## Language-Specific Considerations

Keep language-specific logic isolated in `languages/` modules:
- **Rust**: Module boundaries, pub visibility, trait patterns
- **Java**: Package structure, annotation patterns, Spring-specific concerns
- **Python**: Package/module distinction, import patterns
- **C#**: Namespace organization, attribute patterns
- **JavaScript**: Module systems, framework patterns

## Success Metrics

1. **Utility**: Developers actually read and reference the generated docs
2. **Accuracy**: Documentation reflects actual system behavior and concerns
3. **Maintainability**: Docs stay useful as code evolves
4. **Efficiency**: Tool runs fast enough for regular use
5. **Discovery**: Surfaces insights developers wouldn't have found otherwise

## Key Design Principles

1. **Human edits are sacred** - Always preserve protected regions
2. **Context is king** - More context leads to better insights
3. **Practical over comprehensive** - Focus on what developers actually need
4. **Progressive enhancement** - Basic docs first, intelligence second
5. **Language-agnostic core** - Keep language specifics isolated
6. **Prompt carefully** - Avoid steering, encourage discovery
7. **Mirror mental models** - Documentation structure should match how developers think about the system

## Getting Started

The next session should focus on implementing the package-level analysis and batch LLM processing, starting with refactoring the `CodeParser` and `DocGenerator` to work with consolidated package data rather than individual files.