use anyhow::Result;
use clap::{Args, Subcommand};

use browserx_core::{get_cookies, GetCookiesOptions};

#[derive(Args)]
pub struct VaultArgs {
    #[command(subcommand)]
    pub command: VaultCommand,
}

#[derive(Subcommand)]
pub enum VaultCommand {
    /// Store cookies in the encrypted vault
    Store {
        /// URL to extract and store cookies for
        #[arg(long)]
        url: String,

        /// Time-to-live (e.g., "24h", "7d", "1h30m")
        #[arg(long, default_value = "24h")]
        ttl: String,

        /// Browser(s) to extract from
        #[arg(long, value_delimiter = ',')]
        browser: Vec<String>,

        /// Label for this vault entry
        #[arg(long)]
        label: Option<String>,
    },

    /// Retrieve cookies from the vault
    Get {
        /// URL to retrieve cookies for
        #[arg(long)]
        url: String,
    },

    /// List all vault entries
    List,

    /// Remove expired entries and clean up
    Clean,

    /// Remove a specific vault entry
    Remove {
        /// URL or label to remove
        #[arg(long)]
        url: String,
    },
}

pub fn run(args: VaultArgs, format: &str) -> Result<()> {
    match args.command {
        VaultCommand::Store {
            url,
            ttl,
            browser,
            label,
        } => {
            let browsers = browser
                .iter()
                .map(|s| s.parse())
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|e: String| anyhow::anyhow!(e))?;

            let options = GetCookiesOptions {
                url: url.clone(),
                browsers,
                ..Default::default()
            };

            let result = get_cookies(options);

            if result.cookies.is_empty() {
                eprintln!("No cookies found for {url}");
                std::process::exit(1);
            }

            let vault = browserx_vault::Vault::open_or_create()?;
            vault.store(&url, &result.cookies, &ttl, label.as_deref())?;

            let count = result.cookies.len();
            eprintln!("Stored {count} cookies for {url} (TTL: {ttl})");

            Ok(())
        }

        VaultCommand::Get { url } => {
            let vault = browserx_vault::Vault::open_or_create()?;
            let cookies = vault.get(&url)?;

            if cookies.is_empty() {
                eprintln!("No vault entry found for {url}");
                std::process::exit(1);
            }

            crate::output::render_cookies(&cookies, format)?;
            Ok(())
        }

        VaultCommand::List => {
            let vault = browserx_vault::Vault::open_or_create()?;
            let entries = vault.list()?;

            if entries.is_empty() {
                println!("Vault is empty.");
                return Ok(());
            }

            match format {
                "json" => {
                    println!("{}", serde_json::to_string_pretty(&entries)?);
                }
                _ => {
                    println!("Vault entries:");
                    for entry in &entries {
                        println!(
                            "  {} -- {} cookies, expires: {}",
                            entry.url, entry.cookie_count, entry.expires_at
                        );
                    }
                }
            }

            Ok(())
        }

        VaultCommand::Clean => {
            let vault = browserx_vault::Vault::open_or_create()?;
            let removed = vault.clean()?;
            eprintln!("Removed {removed} expired entries.");
            Ok(())
        }

        VaultCommand::Remove { url } => {
            let vault = browserx_vault::Vault::open_or_create()?;
            vault.remove(&url)?;
            eprintln!("Removed vault entry for {url}");
            Ok(())
        }
    }
}
