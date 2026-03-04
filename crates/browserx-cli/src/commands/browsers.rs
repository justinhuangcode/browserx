use anyhow::Result;
use clap::Args;
use serde_json::json;

use browserx_core::providers;

#[derive(Args)]
pub struct BrowsersArgs {
    /// Only check specific browser(s)
    #[arg(long, value_delimiter = ',')]
    pub filter: Vec<String>,
}

pub fn run(args: BrowsersArgs, format: &str) -> Result<()> {
    let detected = providers::detect_browsers();

    let browsers: Vec<_> = detected
        .iter()
        .filter(|b| {
            if args.filter.is_empty() {
                true
            } else {
                args.filter
                    .iter()
                    .any(|f| f.eq_ignore_ascii_case(&format!("{:?}", b)))
            }
        })
        .collect();

    match format {
        "json" => {
            let json_output: Vec<_> = browsers
                .iter()
                .map(|b| {
                    json!({
                        "name": format!("{:?}", b).to_lowercase(),
                        "display_name": b.display_name(),
                        "chromium_based": b.is_chromium_based(),
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&json_output)?);
        }
        _ => {
            if browsers.is_empty() {
                println!("No browsers detected.");
            } else {
                println!("Detected browsers:");
                for b in &browsers {
                    let engine = if b.is_chromium_based() {
                        " (Chromium)"
                    } else {
                        ""
                    };
                    println!("  - {}{}", b.display_name(), engine);
                }
            }
        }
    }

    Ok(())
}
