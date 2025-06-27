# DocForge

![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)
![Status](https://img.shields.io/badge/status-alpha-orange.svg)
![Language](https://img.shields.io/badge/rust-1.70+-orange.svg)

**The Documentation Generator That Actually Stays Current**

Docs rot. Code evolves. Humans forget.

DocForge breaks this cycle by creating **living documentation** that preserves your edits while staying synchronized with your codebase. The knowledge doesn't live in model weights—it lives in excellent, searchable, version-controlled documentation that serves both your team and your tools.

## The Problem We Solve

**The Onboarding Crisis**: New engineers spend weeks interrupting busy teammates with questions that should be documented.

**The Documentation Death Spiral**: Traditional docs become outdated immediately and stay that way.

**The Context Gap**: AI assistants lack the architectural intent and tribal knowledge behind your code decisions.

## How DocForge Works

DocForge uses AST-aware diffing to detect code changes, then regenerates only the affected docs—without touching your edits. Here's the magic:

1. **Content Hashing**: Each section includes a content hash linked to the code it documents
2. **Protected Regions**: Humans can mark sections as `<!-- PROTECTED -->` to prevent regeneration
3. **Human Edits Are First-Class**: Edits persist and enrich future generations. What you fix today becomes tomorrow's onboarding win
4. **Incremental Updates**: Only regenerates what actually changed in the codebase

### Human Edit Protection

Protect your edits with simple markdown comments that work everywhere:

```markdown
<!-- PROTECTED: Architecture Decision -->
This service uses event sourcing because we need perfect audit trails 
for regulatory compliance. The performance trade-off is acceptable 
because queries are read-heavy and we can use CQRS projections.
<!-- /PROTECTED -->
```

**Why This Matters:**
- **Hugo/Jekyll Compatible**: Standard HTML comments don't break static site generators
- **AI Signal**: Coding assistants and future integrations (Copilot, GPT plugins) recognize this as carefully curated, high-value context
- **Human Lazy Test**: If someone bothered to protect it, it's important or frequently misunderstood

## Example Output Structure

DocForge generates structured markdown with metadata headers:

```markdown
---
generated_from: src/analytics/mod.rs
last_updated: <date>
protected_sections: ["architecture-decision", "performance-notes"]
team_metadata: staging-env-config
---

# Analytics Service

<!-- PROTECTED: Architecture Decision -->
This service uses event sourcing because...
<!-- /PROTECTED -->

## Public API

Generated documentation for public functions...
```

This header is generated and updated by DocForge, but you can extend it with your own fields—it's just frontmatter.

## Quick Start

```bash
# Initialize documentation structure
docforge init

# Generate initial documentation  
docforge generate

# Update only changed sections
docforge sync

# Validate documentation health
docforge validate

# Export for static sites
docforge publish --format hugo
```

## Why Not Just Prompt GPT?

**DocForge creates structured, version-controlled knowledge** that evolves with your codebase:

- **Persistent**: Your edits and context survive code changes
- **Discoverable**: Instead of asking "what does the analytics service do?" every few months, your team browses `docs/services/analytics/README.md`, which has up-to-date API signatures, data flow diagrams, and a note on why event sourcing was chosen in 2022
- **Collaborative**: Team knowledge accumulates in version control, not individual chat histories  
- **Cumulative**: Each human edit enriches the documentation for everyone who comes after

## Technical Architecture

**Parser Engine**: Tree-sitter provides consistent, language-agnostic code analysis

**Edit Preservation**: AST diffs + content hashing let DocForge regenerate only what changed—without touching your carefully written sections

**Pattern Recognition**: DocForge detects recurring human edits and uses them to fine-tune template output—no cloud model training, no external API calls

**Output Format**: Pure markdown with metadata headers for static site generator compatibility

## FAQ

**Q: How do you avoid AI hallucinations?**
A: DocForge generates structure and relationships from actual code. AI only fills templates with factual information extracted from the AST.

**Q: What about legacy codebases?**
A: Works on any code Tree-sitter can parse. No special comments or annotations required.

**Q: Can I customize the output format?**
A: Yes, through `docforge.toml` templates and per-project style guides.

**Q: What if the AI consistently misunderstands something?**
A: Mark it `<!-- PROTECTED -->` once and it stays fixed forever. Your correction becomes part of the permanent knowledge base.

**Q: Does this send my code to external APIs?**
A: No. All processing happens locally. The only external calls are optional AI API requests for content generation, which you control completely.

## Roadmap

### Phase 1: Core Engine (MVP)
- [ ] Basic CLI interface
- [ ] Markdown template system
- [ ] Tree-sitter parsing for Rust
- [ ] AST-aware diff engine
- [ ] Protected region preservation
- [ ] Documentation validation

### Phase 2: Multi-Language Intelligence  
- [ ] Python, Java, JavaScript support
- [ ] Cross-module relationship detection
- [ ] Static site generator exports
- [ ] Documentation health analytics

### Phase 3: Team Collaboration
- [ ] Web review interface
- [ ] Team templates and style guides
- [ ] Integration APIs for development tools
- [ ] Advanced pattern recognition from team corrections

## CI Integration

DocForge works seamlessly in your existing workflows:

```yaml
# .github/workflows/docs.yml
- name: Validate Documentation
  run: docforge validate --strict
  
- name: Check for Stale Docs
  run: docforge sync --dry-run --fail-on-changes
```

## Project Structure

```
docs/
├── architecture/          # System overviews and decisions
├── services/             # Per-service documentation
│   ├── analytics/
│   │   ├── README.md    # Service overview
│   │   ├── api.md       # Public interfaces
│   │   └── internals/   # Implementation details
├── guides/              # Human-authored tutorials
└── decisions/           # Architectural decision records
```

## The Vision

DocForge isn't just a documentation tool—it's the foundation for **human-AI collaboration in software development**. 

Your curated documentation becomes the context layer that makes every AI coding assistant dramatically more effective. Instead of generic suggestions, they understand your architecture, your patterns, your decisions.

**"Bend the information space around your development process"**

## License & Features

| Feature | Open Source | Commercial |
|---------|-------------|------------|
| Core documentation generation | ✅ | ✅ |
| Protected regions | ✅ | ✅ |
| Multi-language support | ✅ | ✅ |
| Team collaboration | ❌ | ✅ |
| Advanced integrations | ❌ | ✅ |
| Priority support | ❌ | ✅ |

**Open Source**: Apache 2.0 for core documentation generation  
**Commercial**: Team collaboration and enterprise features

---

Built with Rust for performance. Designed for humans who actually have to maintain the docs.

**[Follow Development](https://github.com/your-org/docforge) • [Join Discussions](https://github.com/your-org/docforge/discussions)**
