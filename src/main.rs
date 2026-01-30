use clap::Parser;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use miette::{Context, IntoDiagnostic, Result};
use serde::Deserialize;
use std::fs;
use std::io::{Read, Write};
use std::process::Command;

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

#[derive(Parser, Debug)]
#[command(author, version, about = "Update mq to the latest version", long_about = None)]
struct Args {
    /// Target version to install (defaults to latest)
    #[arg(short = 't', long = "target")]
    target_version: Option<String>,

    /// Force reinstall even if already up-to-date
    #[arg(short, long)]
    force: bool,

    /// Show current version
    #[arg(long)]
    current: bool,
}

fn get_mq_path() -> Result<std::path::PathBuf> {
    let output = Command::new("which")
        .arg("mq")
        .output()
        .into_diagnostic()
        .wrap_err("Failed to find mq in PATH")?;

    if !output.status.success() {
        return Err(miette::miette!(
            "mq command not found in PATH. Make sure mq is installed."
        ));
    }

    let path_str = String::from_utf8(output.stdout)
        .into_diagnostic()
        .wrap_err("Failed to parse mq path")?;

    Ok(std::path::PathBuf::from(path_str.trim()))
}

fn get_mq_version() -> Result<String> {
    let output = Command::new("mq")
        .arg("--version")
        .output()
        .into_diagnostic()
        .wrap_err("Failed to execute 'mq --version'. Make sure mq is installed and in PATH.")?;

    if !output.status.success() {
        return Err(miette::miette!("mq command failed"));
    }

    let version_output = String::from_utf8(output.stdout)
        .into_diagnostic()
        .wrap_err("Failed to parse mq version output")?;

    // Parse version from output like "mq 0.5.12" or "mq-cli 0.5.12"
    let version = version_output
        .split_whitespace()
        .last()
        .ok_or_else(|| miette::miette!("Could not parse version from mq output"))?
        .trim()
        .to_string();

    Ok(version)
}

fn get_latest_release(target_version: Option<&String>) -> Result<Release> {
    let url = if let Some(version) = target_version {
        let tag = if version.starts_with('v') {
            version.clone()
        } else {
            format!("v{}", version)
        };
        format!(
            "https://api.github.com/repos/harehare/mq/releases/tags/{}",
            tag
        )
    } else {
        "https://api.github.com/repos/harehare/mq/releases/latest".to_string()
    };

    let client = reqwest::blocking::Client::builder()
        .user_agent("mq-update")
        .build()
        .into_diagnostic()?;

    let response = client
        .get(&url)
        .send()
        .into_diagnostic()
        .wrap_err("Failed to fetch release information from GitHub")?;

    if !response.status().is_success() {
        return Err(miette::miette!(
            "Failed to fetch release: HTTP {}",
            response.status()
        ));
    }

    response
        .json::<Release>()
        .into_diagnostic()
        .wrap_err("Failed to parse release information")
}

fn get_target_arch() -> &'static str {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return "aarch64-apple-darwin";

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return "x86_64-apple-darwin";

    #[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "musl"))]
    return "x86_64-unknown-linux-musl";

    #[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
    return "x86_64-unknown-linux-gnu";

    #[cfg(all(
        target_os = "linux",
        target_arch = "x86_64",
        not(any(target_env = "musl", target_env = "gnu"))
    ))]
    return "x86_64-unknown-linux-gnu";

    #[cfg(all(target_os = "linux", target_arch = "aarch64", target_env = "musl"))]
    return "aarch64-unknown-linux-musl";

    #[cfg(all(target_os = "linux", target_arch = "aarch64", target_env = "gnu"))]
    return "aarch64-unknown-linux-gnu";

    #[cfg(all(
        target_os = "linux",
        target_arch = "aarch64",
        not(any(target_env = "musl", target_env = "gnu"))
    ))]
    return "aarch64-unknown-linux-gnu";

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return "x86_64-pc-windows-msvc.exe";

    #[cfg(not(any(
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "windows", target_arch = "x86_64")
    )))]
    compile_error!("Unsupported platform");
}

fn download_and_replace(download_url: &str, mq_path: &std::path::Path, force: bool) -> Result<()> {
    if !force {
        println!("\n{}", "┌────────────────────────────────────────┐".cyan());
        println!("{}", "│  The binary will be replaced          │".cyan());
        println!("{}", "└────────────────────────────────────────┘".cyan());
        print!("\n{} ", "Do you want to continue? [Y/n]".bold());
        std::io::stdout().flush().into_diagnostic()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input).into_diagnostic()?;

        if !input.trim().is_empty() && !input.trim().eq_ignore_ascii_case("y") {
            println!("\n{}", "✗ Update cancelled".yellow());
            return Ok(());
        }
    }

    println!("\n{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".cyan());
    println!("{}", "  Downloading binary...".bold());
    println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".cyan());

    let client = reqwest::blocking::Client::builder()
        .user_agent("mq-update")
        .build()
        .into_diagnostic()?;

    let mut response = client
        .get(download_url)
        .send()
        .into_diagnostic()
        .wrap_err("Failed to download binary")?;

    if !response.status().is_success() {
        return Err(miette::miette!(
            "Failed to download binary: HTTP {}",
            response.status()
        ));
    }

    let total_size = response.content_length().unwrap_or(0);

    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .into_diagnostic()?
            .progress_chars("█▓▒░ ")
    );

    let mut buffer = Vec::new();
    let mut downloaded: u64 = 0;

    loop {
        let mut chunk = vec![0; 8192];
        match response.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                buffer.extend_from_slice(&chunk[..n]);
                downloaded += n as u64;
                pb.set_position(downloaded);
            }
            Err(e) => return Err(miette::miette!("Download failed: {}", e)),
        }
    }

    pb.finish_with_message("Download complete!");

    println!("\n{} {}", "✓".green().bold(), "Download complete!".green());

    // Create backup
    println!("\n{}", "Creating backup...".dimmed());
    let backup_path = mq_path.with_extension("bak");
    if mq_path.exists() {
        fs::copy(mq_path, &backup_path)
            .into_diagnostic()
            .wrap_err("Failed to create backup")?;
        println!(
            "{} Backup created: {}",
            "✓".green(),
            backup_path.display().to_string().dimmed()
        );
    }

    // Write new binary
    println!("{}", "Replacing binary...".dimmed());
    fs::write(mq_path, &buffer)
        .into_diagnostic()
        .wrap_err("Failed to write new binary")?;

    // Set executable permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(mq_path).into_diagnostic()?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(mq_path, perms).into_diagnostic()?;
    }

    // Remove backup if update succeeded
    if backup_path.exists() {
        let _ = fs::remove_file(&backup_path);
    }

    println!(
        "{} {}",
        "✓".green().bold(),
        "Binary replaced successfully!".green()
    );

    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();

    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());

    writeln!(
        out,
        "\n{}",
        "╔══════════════════════════════════════╗".cyan().bold()
    )
    .into_diagnostic()?;
    writeln!(
        out,
        "{}",
        "║       mq Update Manager              ║".cyan().bold()
    )
    .into_diagnostic()?;
    writeln!(
        out,
        "{}",
        "╚══════════════════════════════════════╝".cyan().bold()
    )
    .into_diagnostic()?;

    let mq_path = get_mq_path()?;
    let current_version = get_mq_version()?;

    if args.current {
        writeln!(
            out,
            "\n{} {}",
            "📦 Current mq version:".bold(),
            current_version.green().bold()
        )
        .into_diagnostic()?;
        out.flush().into_diagnostic()?;
        return Ok(());
    }

    writeln!(
        out,
        "\n{} {}",
        "📦 Current version:".bold(),
        current_version.cyan()
    )
    .into_diagnostic()?;

    write!(out, "{}", "🔍 Checking for updates...".dimmed()).into_diagnostic()?;
    out.flush().into_diagnostic()?;

    let release = get_latest_release(args.target_version.as_ref())?;
    let target_version = release.tag_name.trim_start_matches('v');

    writeln!(out, " {}", "Done!".green()).into_diagnostic()?;
    writeln!(
        out,
        "{} {}",
        "📦 Latest version: ".bold(),
        target_version.green().bold()
    )
    .into_diagnostic()?;

    if !args.force && current_version == target_version {
        writeln!(
            out,
            "\n{}",
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".cyan()
        )
        .into_diagnostic()?;
        writeln!(out, "{}", "  ✓ Already up-to-date!".green().bold()).into_diagnostic()?;
        writeln!(out, "{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".cyan()).into_diagnostic()?;
        out.flush().into_diagnostic()?;
        return Ok(());
    }

    let target_arch = get_target_arch();
    let asset_name = format!("mq-{}", target_arch);

    let asset = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .ok_or_else(|| {
            miette::miette!(
                "Could not find binary for architecture: {}. Available assets: {}",
                target_arch,
                release
                    .assets
                    .iter()
                    .map(|a| &a.name)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?;

    writeln!(out, "\n{} {}", "🔗 Asset:".dimmed(), asset.name.dimmed()).into_diagnostic()?;
    out.flush().into_diagnostic()?;

    download_and_replace(&asset.browser_download_url, &mq_path, args.force)?;

    // Verify update
    write!(out, "\n{}", "Verifying installation...".dimmed()).into_diagnostic()?;
    out.flush().into_diagnostic()?;

    let new_version = get_mq_version()?;

    writeln!(out, " {}", "Done!".green()).into_diagnostic()?;
    writeln!(
        out,
        "\n{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".cyan()
    )
    .into_diagnostic()?;

    if new_version == target_version {
        writeln!(
            out,
            "{}",
            format!("  ✓ Successfully updated to version {}", target_version)
                .green()
                .bold()
        )
        .into_diagnostic()?;
        writeln!(
            out,
            "  {} → {}",
            current_version.dimmed(),
            new_version.green().bold()
        )
        .into_diagnostic()?;
    } else {
        writeln!(
            out,
            "{}",
            format!(
                "  ⚠ Update completed but version is {}. Expected {}",
                new_version, target_version
            )
            .yellow()
            .bold()
        )
        .into_diagnostic()?;
        writeln!(out, "{}", "  Try running mq again to verify.".dimmed()).into_diagnostic()?;
    }
    writeln!(out, "{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".cyan()).into_diagnostic()?;
    out.flush().into_diagnostic()?;

    Ok(())
}
