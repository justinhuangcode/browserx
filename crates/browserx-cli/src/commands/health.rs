use anyhow::Result;
use clap::Args;

use browserx_core::{check_health, get_cookies, GetCookiesOptions, HealthStatus};

#[derive(Args)]
pub struct HealthArgs {
    /// URL to check session health for
    #[arg(long)]
    pub url: String,

    /// Browser(s) to check
    #[arg(long, value_delimiter = ',')]
    pub browser: Vec<String>,

    /// Cookie names to focus on
    #[arg(long, value_delimiter = ',')]
    pub names: Vec<String>,
}

pub fn run(args: HealthArgs, format: &str) -> Result<()> {
    let browsers = args
        .browser
        .iter()
        .map(|s| s.parse())
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e: String| anyhow::anyhow!(e))?;

    let options = GetCookiesOptions {
        url: args.url.clone(),
        browsers,
        names: args.names,
        ..Default::default()
    };

    let result = get_cookies(options);
    let health = check_health(&result.cookies, &args.url);

    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&health)?);
        }
        _ => {
            let status_indicator = match health.status {
                HealthStatus::Healthy => "[OK]",
                HealthStatus::Warning => "[!!]",
                HealthStatus::Expired => "[XX]",
                HealthStatus::Empty => "[--]",
            };

            println!("{} {} -- {}", status_indicator, health.url, health.status);
            println!(
                "  Cookies: {} total, {} active, {} expiring soon, {} expired",
                health.total_cookies, health.active_cookies, health.expiring_soon, health.expired
            );

            if !health.details.is_empty() {
                println!();
                for detail in &health.details {
                    let expires = detail.expires_in.as_deref().unwrap_or("session");
                    println!(
                        "  {:20} {:15} {:?} ({})",
                        detail.name, detail.domain, detail.status, expires
                    );
                }
            }

            // Print warnings
            for warning in &result.warnings {
                eprintln!("warning: {warning}");
            }
        }
    }

    Ok(())
}
