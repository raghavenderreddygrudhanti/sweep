mod commands;
mod scanner;
mod cleaners;
mod history;
mod cache;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "sweep")]
#[command(about = "🧹 Fast system cleaner for macOS and Linux. 10x faster than shell-based tools.")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan your system and show what's eating disk space
    Scan {
        /// Path to scan (defaults to home directory)
        #[arg(default_value = "~")]
        path: String,
    },
    /// Clean caches, logs, and junk files
    Clean {
        /// Preview what would be deleted without actually deleting
        #[arg(long)]
        dry_run: bool,
    },
    /// Clean AI/ML caches (HuggingFace, Ollama, torch, models)
    Ai {
        /// Preview only
        #[arg(long)]
        dry_run: bool,
    },
    /// Clean Docker images, volumes, and build cache
    Docker {
        /// Preview only
        #[arg(long)]
        dry_run: bool,
    },
    /// Clean developer build artifacts (node_modules, target, .venv)
    Dev {
        /// Preview only
        #[arg(long)]
        dry_run: bool,

        /// Minimum age in days before cleaning (default: 7)
        #[arg(long, default_value = "7")]
        older_than: u64,
    },
    /// Uninstall apps and all their remnants
    Uninstall {
        /// Preview only
        #[arg(long)]
        dry_run: bool,
    },
    /// Optimize system (flush DNS, rebuild caches, refresh services)
    Optimize {
        /// Preview only
        #[arg(long)]
        dry_run: bool,
    },
    /// Find and remove installer files (.dmg, .pkg)
    Installer {
        /// Preview only
        #[arg(long)]
        dry_run: bool,
    },
    /// Show real-time system status
    Status,
    /// Show operation history
    History,
    /// Generate shell completions
    Completion {
        /// Shell type (bash, zsh, fish)
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Scan { path }) => commands::scan::run(&path),
        Some(Commands::Clean { dry_run }) => commands::clean::run(dry_run),
        Some(Commands::Ai { dry_run }) => commands::ai::run(dry_run),
        Some(Commands::Docker { dry_run }) => commands::docker::run(dry_run),
        Some(Commands::Dev { dry_run, older_than }) => commands::dev::run(dry_run, older_than),
        Some(Commands::Uninstall { dry_run }) => commands::uninstall::run(dry_run),
        Some(Commands::Optimize { dry_run }) => commands::optimize::run(dry_run),
        Some(Commands::Installer { dry_run }) => commands::installer::run(dry_run),
        Some(Commands::Status) => commands::status::run(),
        Some(Commands::History) => {
            println!("\n  \x1b[1mOperation History\x1b[0m\n");
            history::show_history();
            println!();
        },
        Some(Commands::Completion { shell }) => {
            use clap::CommandFactory;
            clap_complete::generate(shell, &mut Cli::command(), "sweep", &mut std::io::stdout());
        },
        None => commands::interactive::run(),
    }
}
