use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

mod commands;
mod output;

#[derive(Parser)]
#[command(
    name = "browserx",
    about = "Extract browser cookies from any browser.\n\nDesigned for AI agents, CLI automation, and programmatic authenticated web access.",
    version,
    after_help = "EXAMPLES:\n  \
        browserx get --url https://github.com\n  \
        browserx get --url https://github.com --browser chrome --format curl\n  \
        browserx get --url https://x.com --names session,token --format env\n  \
        browserx browsers\n  \
        browserx health --url https://github.com\n  \
        browserx vault store --url https://github.com --ttl 24h\n  \
        browserx vault get --url https://github.com"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output format (overrides per-command defaults)
    #[arg(long, global = true, value_parser = ["json", "curl", "netscape", "env", "table"])]
    format: Option<String>,

    /// Enable verbose logging (repeat for more: -v, -vv, -vvv)
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Suppress all output except data
    #[arg(short, long, global = true)]
    quiet: bool,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Extract cookies from browser(s)
    Get(commands::get::GetArgs),

    /// List detected browsers and profiles
    Browsers(commands::browsers::BrowsersArgs),

    /// Check session health for a URL
    Health(commands::health::HealthArgs),

    /// Encrypted cookie vault (store, retrieve, manage)
    Vault(commands::vault::VaultArgs),
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Configure logging based on verbosity
    let filter = match cli.verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(filter)),
        )
        .with_target(false)
        .with_writer(std::io::stderr)
        .init();

    let format = cli.format.as_deref().unwrap_or("json");

    match cli.command {
        Commands::Get(args) => commands::get::run(args, format, cli.quiet),
        Commands::Browsers(args) => commands::browsers::run(args, format),
        Commands::Health(args) => commands::health::run(args, format),
        Commands::Vault(args) => commands::vault::run(args, format),
    }
}
