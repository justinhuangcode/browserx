use anyhow::Result;
use clap::Args;

use browserx_core::{get_cookies, BrowserName, GetCookiesOptions, MergeMode};

use crate::output;

#[derive(Args)]
pub struct GetArgs {
    /// Target URL to extract cookies for
    #[arg(long, required_unless_present_any = ["inline_json", "inline_base64", "inline_file"])]
    pub url: Option<String>,

    /// Browser(s) to extract from (comma-separated, default: auto-detect all)
    #[arg(long, value_delimiter = ',', env = "BROWSEREX_BROWSERS")]
    pub browser: Vec<String>,

    /// Additional origins to include (e.g., OAuth/SSO domains)
    #[arg(long, value_delimiter = ',')]
    pub origins: Vec<String>,

    /// Filter cookies by name (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub names: Vec<String>,

    /// Merge mode: 'merge' (combine all) or 'first' (first success)
    #[arg(long, default_value = "merge", env = "BROWSEREX_MODE")]
    pub mode: String,

    /// Include expired cookies
    #[arg(long)]
    pub include_expired: bool,

    /// Timeout for OS keychain operations (ms)
    #[arg(long, default_value = "5000")]
    pub timeout: u64,

    // -- Inline sources --
    /// Inline cookie JSON payload
    #[arg(long)]
    pub inline_json: Option<String>,

    /// Inline base64-encoded cookie payload
    #[arg(long)]
    pub inline_base64: Option<String>,

    /// Path to a cookie JSON export file
    #[arg(long)]
    pub inline_file: Option<String>,

    // -- Profile overrides --
    /// Chrome profile name (default: "Default")
    #[arg(long, env = "BROWSEREX_CHROME_PROFILE")]
    pub chrome_profile: Option<String>,

    /// Edge profile name
    #[arg(long, env = "BROWSEREX_EDGE_PROFILE")]
    pub edge_profile: Option<String>,

    /// Firefox profile name
    #[arg(long, env = "BROWSEREX_FIREFOX_PROFILE")]
    pub firefox_profile: Option<String>,

    /// Brave profile name
    #[arg(long, env = "BROWSEREX_BRAVE_PROFILE")]
    pub brave_profile: Option<String>,

    /// Direct path to Safari Cookies.binarycookies
    #[arg(long)]
    pub safari_cookies_file: Option<String>,
}

pub fn run(args: GetArgs, format: &str, quiet: bool) -> Result<()> {
    let browsers: Vec<BrowserName> = args
        .browser
        .iter()
        .map(|s| s.parse::<BrowserName>())
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| anyhow::anyhow!(e))?;

    let mode: MergeMode = args.mode.parse().map_err(|e: String| anyhow::anyhow!(e))?;

    let options = GetCookiesOptions {
        url: args.url.unwrap_or_default(),
        origins: args.origins,
        names: args.names,
        browsers,
        mode,
        include_expired: args.include_expired,
        timeout_ms: args.timeout,
        inline_json: args.inline_json,
        inline_base64: args.inline_base64,
        inline_file: args.inline_file,
        chrome_profile: args.chrome_profile,
        edge_profile: args.edge_profile,
        firefox_profile: args.firefox_profile,
        brave_profile: args.brave_profile,
        safari_cookies_file: args.safari_cookies_file,
    };

    let result = get_cookies(options);

    // Print warnings to stderr
    if !quiet {
        for warning in &result.warnings {
            eprintln!("warning: {warning}");
        }
    }

    // Output cookies in requested format
    output::render_cookies(&result.cookies, format)?;

    // Exit with code 1 if no cookies found
    if result.cookies.is_empty() {
        std::process::exit(1);
    }

    Ok(())
}
