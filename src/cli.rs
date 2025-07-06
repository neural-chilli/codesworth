use clap::{Parser, Subcommand};
use std::path::PathBuf;
use anyhow::Result;

use crate::core::Engine;

#[derive(Parser)]
#[command(name = "codesworth")]
#[command(about = "The Documentation Generator That Actually Stays Current")]
#[command(version)]
pub struct Cli {
    /// Path to configuration file
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Enable verbose logging
    #[arg(short, long)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize documentation structure
    Init {
        /// Target directory (defaults to current directory)
        #[arg(short, long)]
        path: Option<PathBuf>,

        /// Skip interactive configuration
        #[arg(long)]
        non_interactive: bool,
    },

    /// Generate initial documentation
    Generate {
        /// Source directory to analyze
        #[arg(short, long)]
        source: Option<PathBuf>,

        /// Output directory for documentation
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Force regeneration of all docs
        #[arg(long)]
        force: bool,
    },

    /// Update only changed sections
    Sync {
        /// Dry run - show what would be updated
        #[arg(long)]
        dry_run: bool,

        /// Fail if changes would be made (useful for CI)
        #[arg(long)]
        fail_on_changes: bool,
    },

    /// Validate documentation health
    Validate {
        /// Use strict validation rules
        #[arg(long)]
        strict: bool,
    },

    /// Export for static sites
    Publish {
        /// Output format (hugo, jekyll, gitbook)
        #[arg(long, default_value = "hugo")]
        format: String,

        /// Output directory
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

impl Cli {
    pub async fn execute(self, mut engine: Engine) -> Result<()> {
        match self.command {
            Commands::Init { path, non_interactive } => {
                engine.init(path, non_interactive).await
            }
            Commands::Generate { source, output, force } => {
                engine.generate(source, output, force).await
            }
            Commands::Sync { dry_run, fail_on_changes } => {
                engine.sync(dry_run, fail_on_changes).await
            }
            Commands::Validate { strict } => {
                engine.validate(strict).await
            }
            Commands::Publish { format, output } => {
                engine.publish(&format, output).await
            }
        }
    }
}