use clap::Parser;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use miette::{Context, IntoDiagnostic, Result};
use serde::Deserialize;
use std::fs;
use std::io::{Read, Write};
use std::process::Command;
use std::time::Duration;

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
    /// Subcommand name to install/update (e.g., "check" for mq-check)
    subcommand: Option<String>,

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

fn get_binary_path(binary_name: &str) -> Result<Option<std::path::PathBuf>> {
    let output = Command::new("which")
        .arg(binary_name)
        .output()
        .into_diagnostic()
        .wrap_err(format!("Failed to find {} in PATH", binary_name))?;

    if !output.status.success() {
        return Ok(None);
    }

    let path_str = String::from_utf8(output.stdout)
        .into_diagnostic()
        .wrap_err(format!("Failed to parse {} path", binary_name))?;

    Ok(Some(std::path::PathBuf::from(path_str.trim())))
}

fn get_binary_version(binary_name: &str) -> Result<Option<String>> {
    let output = match Command::new(binary_name).arg("--version").output() {
        Ok(output) => output,
        Err(_) => return Ok(None),
    };

    if !output.status.success() {
        return Ok(None);
    }

    let version_output = String::from_utf8(output.stdout)
        .into_diagnostic()
        .wrap_err("Failed to parse version output")?;

    // Parse version from output like "mq 0.5.12" or "mq-check 0.1.0"
    let version = version_output
        .split_whitespace()
        .last()
        .ok_or_else(|| miette::miette!("Could not parse version from output"))?
        .trim()
        .to_string();

    Ok(Some(version))
}

fn get_latest_release(repo: &str, target_version: Option<&String>) -> Result<Release> {
    let url = if let Some(version) = target_version {
        let tag = if version.starts_with('v') {
            version.clone()
        } else {
            format!("v{}", version)
        };
        format!(
            "https://api.github.com/repos/{}/releases/tags/{}",
            repo, tag
        )
    } else {
        format!("https://api.github.com/repos/{}/releases/latest", repo)
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

fn print_logo() {
    println!();
    println!("{}", "           ███╗   ███╗ ██████╗ ".bright_cyan().bold());
    println!("{}", "           ████╗ ████║██╔═══██╗".bright_cyan().bold());
    println!("{}", "           ██╔████╔██║██║   ██║".bright_cyan().bold());
    println!("{}", "           ██║╚██╔╝██║██║▄▄ ██║".bright_cyan().bold());
    println!("{}", "           ██║ ╚═╝ ██║╚██████╔╝".bright_cyan().bold());
    println!(
        "{}",
        "           ╚═╝     ╚═╝ ╚══════╝ ".bright_cyan().bold()
    );
    println!();
    println!("{}", "        Update Manager for mq".bright_white());
    println!("{}", "    ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_black());
    println!();
}

fn download_and_replace(
    download_url: &str,
    mq_path: &std::path::Path,
    force: bool,
    is_new_install: bool,
) -> Result<()> {
    if !force && !is_new_install {
        println!();
        println!(
            "{}",
            "  ╭────────────────────────────────────────╮".bright_cyan()
        );
        println!(
            "{}",
            "  │                                        │".bright_cyan()
        );
        println!(
            "  │  {}    │",
            "⚠  The binary will be replaced    ".bright_yellow().bold()
        );
        println!(
            "{}",
            "  │                                        │".bright_cyan()
        );
        println!(
            "{}",
            "  ╰────────────────────────────────────────╯".bright_cyan()
        );
        print!(
            "\n  {} {} ",
            "❯".bright_cyan().bold(),
            "Do you want to continue? [Y/n]".bold()
        );
        std::io::stdout().flush().into_diagnostic()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input).into_diagnostic()?;

        if !input.trim().is_empty() && !input.trim().eq_ignore_ascii_case("y") {
            println!();
            println!(
                "  {} {}",
                "✗".bright_red().bold(),
                "Update cancelled".bright_red()
            );
            println!();
            return Err(miette::miette!("Update cancelled by user"));
        }
    }

    println!();
    println!(
        "{}",
        "  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_cyan()
    );
    println!("  📦 {}", "Downloading binary...".bright_white().bold());
    println!(
        "{}",
        "  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_cyan()
    );
    println!();

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
            .template("  {spinner:.bright_cyan} {msg} [{bar:40.bright_cyan/blue}] {bytes}/{total_bytes} {elapsed_precise}")
            .into_diagnostic()?
            .progress_chars("━╸─")
    );
    pb.set_message("Downloading".to_string());

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

    pb.finish_and_clear();

    println!(
        "\n  {} {}\n",
        "✓".bright_green().bold(),
        "Download complete!".bright_green().bold()
    );

    // Create backup
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("  {spinner:.bright_cyan} {msg}")
            .into_diagnostic()?,
    );
    spinner.set_message("Creating backup...".to_string());
    spinner.enable_steady_tick(Duration::from_millis(80));

    let backup_path = mq_path.with_extension("bak");
    if mq_path.exists() {
        fs::copy(mq_path, &backup_path)
            .into_diagnostic()
            .wrap_err("Failed to create backup")?;
        spinner.finish_and_clear();
        println!(
            "  {} Backup created: {}",
            "✓".bright_green().bold(),
            backup_path.display().to_string().bright_black()
        );
    } else {
        spinner.finish_and_clear();
    }

    // Write to temporary file first to avoid corrupting the running binary
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("  {spinner:.bright_cyan} {msg}")
            .into_diagnostic()?,
    );
    spinner.set_message("Replacing binary...".to_string());
    spinner.enable_steady_tick(Duration::from_millis(80));
    let temp_path = mq_path.with_extension("tmp");

    // Clean up any existing temp file
    if temp_path.exists() {
        let _ = fs::remove_file(&temp_path);
    }

    fs::write(&temp_path, &buffer)
        .into_diagnostic()
        .wrap_err("Failed to write new binary to temporary file")?;

    // Set executable permissions on Unix before moving
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&temp_path).into_diagnostic()?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&temp_path, perms).into_diagnostic()?;
    }

    // Atomic rename: this replaces the old binary even if it's currently running
    fs::rename(&temp_path, mq_path)
        .into_diagnostic()
        .wrap_err("Failed to replace binary")?;

    // Remove backup if update succeeded
    if backup_path.exists() {
        let _ = fs::remove_file(&backup_path);
    }

    spinner.finish_and_clear();
    println!(
        "  {} {}",
        "✓".bright_green().bold(),
        "Binary replaced successfully!".bright_green().bold()
    );

    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();

    print_logo();

    let (binary_name, repo, display_name) = if let Some(ref sub) = args.subcommand {
        (
            format!("mq-{}", sub),
            format!("harehare/mq-{}", sub),
            format!("mq-{}", sub),
        )
    } else {
        (
            "mq".to_string(),
            "harehare/mq".to_string(),
            "mq".to_string(),
        )
    };

    let binary_path = get_binary_path(&binary_name)?;
    let is_new_install = binary_path.is_none();
    let current_version = if is_new_install {
        None
    } else {
        get_binary_version(&binary_name)?
    };

    if args.current {
        if is_new_install {
            println!(
                "\n  📦 {}\n  {} {}\n  {}\n",
                format!("{} is not installed", display_name)
                    .bright_white()
                    .bold(),
                "├─".bright_black(),
                "not found".bright_yellow().bold(),
                "└─────────────────────────────".bright_black()
            );
        } else if let Some(ref ver) = current_version {
            println!(
                "\n  📦 {}\n  {} {}\n  {}\n",
                format!("Current {} version", display_name)
                    .bright_white()
                    .bold(),
                "├─".bright_black(),
                ver.bright_green().bold(),
                "└─────────────────────────────".bright_black()
            );
        } else {
            println!(
                "\n  📦 {}\n  {} {}\n  {}\n",
                format!("Current {} version", display_name)
                    .bright_white()
                    .bold(),
                "├─".bright_black(),
                "unknown".bright_yellow().bold(),
                "└─────────────────────────────".bright_black()
            );
        }
        return Ok(());
    }

    if is_new_install {
        println!(
            "  📦 {}\n  {} {}\n  {}",
            format!("Installing {}", display_name).bright_white().bold(),
            "├─".bright_black(),
            "not installed yet".bright_yellow().bold(),
            "│".bright_black()
        );
    } else {
        println!(
            "  📦 {}\n  {} {}\n  {}",
            format!("Current {} version", display_name)
                .bright_white()
                .bold(),
            "├─".bright_black(),
            current_version
                .as_deref()
                .unwrap_or("unknown")
                .bright_cyan()
                .bold(),
            "│".bright_black()
        );
    }

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("  {spinner:.bright_cyan} {msg}")
            .unwrap(),
    );
    spinner.set_message("Checking for updates...".to_string());
    spinner.enable_steady_tick(Duration::from_millis(80));

    let release = get_latest_release(&repo, args.target_version.as_ref())?;
    let target_version = release.tag_name.trim_start_matches('v');

    spinner.finish_and_clear();

    println!(
        "  {} {}\n  {}\n  📦 {}\n  {} {}",
        "├─".bright_black(),
        "✓ Update check complete".bright_green(),
        "│".bright_black(),
        "Latest version".bright_white().bold(),
        "└─".bright_black(),
        target_version.bright_green().bold()
    );

    if !is_new_install && !args.force && current_version.as_deref() == Some(target_version) {
        println!(
            "\n{}\n\n    {} {}\n    {} You're running the latest version\n\n{}\n",
            "  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_cyan(),
            "✓".bright_green().bold(),
            "Already up-to-date!".bright_green().bold(),
            "│".bright_black(),
            "  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_cyan()
        );
        return Ok(());
    }

    let target_arch = get_target_arch();
    let asset_name = format!("{}-{}", binary_name, target_arch);

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

    println!(
        "\n  🔗 {}\n  {} {}",
        "Target asset".bright_white().bold(),
        "└─".bright_black(),
        asset.name.bright_black()
    );

    let install_path = if let Some(path) = binary_path {
        path
    } else {
        // Default installation path
        let home = std::env::var("HOME")
            .into_diagnostic()
            .wrap_err("Failed to get HOME directory")?;
        let bin_dir = std::path::PathBuf::from(home).join(".mq").join("bin");
        fs::create_dir_all(&bin_dir)
            .into_diagnostic()
            .wrap_err("Failed to create installation directory")?;
        bin_dir.join(&binary_name)
    };

    download_and_replace(
        &asset.browser_download_url,
        &install_path,
        args.force,
        is_new_install,
    )?;

    if is_new_install {
        println!(
            "\n{}\n\n    {} {}\n    {} Version: {}\n    {} Installed to: {}\n\n{}\n",
            "  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_cyan(),
            "✓".bright_green().bold(),
            format!("Successfully installed {}!", display_name)
                .bright_green()
                .bold(),
            "│".bright_black(),
            target_version.bright_green().bold(),
            "│".bright_black(),
            install_path.display().to_string().bright_black(),
            "  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_cyan()
        );
        println!(
            "  {} Make sure {} is in your PATH\n",
            "⚠".bright_yellow().bold(),
            install_path
                .parent()
                .unwrap()
                .display()
                .to_string()
                .bright_cyan()
        );
    } else {
        println!(
            "\n{}\n\n    {} {}\n    {} Version: {} {} {}\n\n{}\n",
            "  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_cyan(),
            "✓".bright_green().bold(),
            format!("Successfully updated {}!", display_name)
                .bright_green()
                .bold(),
            "│".bright_black(),
            current_version.unwrap_or_default().bright_cyan(),
            "→".bright_white(),
            target_version.bright_green().bold(),
            "  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_cyan()
        );
    }

    Ok(())
}
