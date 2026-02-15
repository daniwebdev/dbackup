use anyhow::{Context, Result, anyhow};
use serde_json::Value;
use std::fs;
use tracing::info;

const GITHUB_API_URL: &str = "https://api.github.com/repos/daniwebdev/dbackup/releases/latest";

#[derive(Debug)]
struct Release {
    tag_name: String,
    download_url: String,
}

fn get_target_triple() -> Result<&'static str> {
    #[cfg(target_os = "linux")]
    #[cfg(target_arch = "x86_64")]
    return Ok("Linux-x86_64");

    #[cfg(target_os = "macos")]
    #[cfg(target_arch = "aarch64")]
    return Ok("Darwin-aarch64");

    #[cfg(target_os = "macos")]
    #[cfg(target_arch = "x86_64")]
    return Ok("Darwin-x86_64");

    #[cfg(not(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64")
    )))]
    {
        Err(anyhow!("Unsupported platform for auto-update"))
    }
}

async fn fetch_latest_release() -> Result<Release> {
    info!("Checking for latest release from GitHub...");

    let client = reqwest::Client::new();
    let response = client
        .get(GITHUB_API_URL)
        .header("User-Agent", "dbackup-updater")
        .send()
        .await
        .context("Failed to fetch latest release from GitHub")?;

    let json: Value = response
        .json()
        .await
        .context("Failed to parse GitHub API response")?;

    let tag_name = json["tag_name"]
        .as_str()
        .ok_or_else(|| anyhow!("No tag_name in GitHub response"))?
        .to_string();

    let target = get_target_triple()?;
    let asset_name = format!("dbackup-{}.tar.gz", target);

    let download_url = json["assets"]
        .as_array()
        .ok_or_else(|| anyhow!("No assets in release"))?
        .iter()
        .find(|asset| {
            asset["name"]
                .as_str()
                .map(|name| name == asset_name)
                .unwrap_or(false)
        })
        .and_then(|asset| asset["browser_download_url"].as_str())
        .ok_or_else(|| anyhow!("Could not find {} in release assets", asset_name))?
        .to_string();

    Ok(Release {
        tag_name,
        download_url,
    })
}

fn version_compare(current: &str, latest: &str) -> std::cmp::Ordering {
    // Compare semantic versions (e.g., "v0.1.0" vs "v0.1.1")
    let current = current.trim_start_matches('v');
    let latest = latest.trim_start_matches('v');

    let current_parts: Vec<&str> = current.split('.').collect();
    let latest_parts: Vec<&str> = latest.split('.').collect();

    for i in 0..3.max(current_parts.len().max(latest_parts.len())) {
        let curr_num: u32 = current_parts
            .get(i)
            .and_then(|p| p.parse().ok())
            .unwrap_or(0);
        let latest_num: u32 = latest_parts
            .get(i)
            .and_then(|p| p.parse().ok())
            .unwrap_or(0);

        match curr_num.cmp(&latest_num) {
            std::cmp::Ordering::Less => return std::cmp::Ordering::Less,
            std::cmp::Ordering::Greater => return std::cmp::Ordering::Greater,
            std::cmp::Ordering::Equal => continue,
        }
    }

    std::cmp::Ordering::Equal
}

pub async fn check_and_show_update(current_version: &str) -> Result<()> {
    if current_version == "develop" {
        return Ok(()); // Don't check updates for develop builds
    }

    match fetch_latest_release().await {
        Ok(release) => {
            match version_compare(current_version, &release.tag_name) {
                std::cmp::Ordering::Less => {
                    println!(
                        "\n✓ A new version is available: {} (current: {})",
                        release.tag_name, current_version
                    );
                    println!("  Run 'dbackup update' to install the latest version");
                }
                std::cmp::Ordering::Equal => {
                    // Same version, don't print anything
                }
                std::cmp::Ordering::Greater => {
                    // Current is newer, don't print anything
                }
            }
            Ok(())
        }
        Err(_) => {
            // Silently ignore update check errors
            Ok(())
        }
    }
}

pub async fn update_binary() -> Result<()> {
    let release = fetch_latest_release().await?;
    info!("Latest release: {}", release.tag_name);

    let temp_dir = std::env::temp_dir();
    let archive_path = temp_dir.join("dbackup-update.tar.gz");
    let extract_dir = temp_dir.join("dbackup-extract");

    // Download the release
    info!("Downloading {} ...", release.download_url);
    let client = reqwest::Client::new();
    let response = client
        .get(&release.download_url)
        .header("User-Agent", "dbackup-updater")
        .send()
        .await
        .context("Failed to download release")?;

    let bytes = response
        .bytes()
        .await
        .context("Failed to read downloaded content")?;

    fs::write(&archive_path, &bytes).context("Failed to write archive file")?;
    info!("Downloaded {} bytes", bytes.len());

    // Extract the archive
    info!("Extracting archive...");
    fs::create_dir_all(&extract_dir).context("Failed to create extraction directory")?;

    let archive_file = fs::File::open(&archive_path).context("Failed to open archive")?;
    let tar = flate2::read::GzDecoder::new(archive_file);
    let mut archive = tar::Archive::new(tar);
    archive
        .unpack(&extract_dir)
        .context("Failed to extract archive")?;

    // Find the binary
    let extracted_binary = extract_dir.join("dbackup");
    if !extracted_binary.exists() {
        return Err(anyhow!("Binary not found in archive"));
    }

    // Get the current binary path
    let current_binary = std::env::current_exe().context("Failed to get current binary path")?;

    // Create backup of current binary
    let backup_path = current_binary.with_extension("old");
    fs::copy(&current_binary, &backup_path)
        .context("Failed to create backup of current binary")?;
    info!("Backed up current binary to: {}", backup_path.display());

    // Replace the binary
    fs::copy(&extracted_binary, &current_binary)
        .context("Failed to replace binary")?;

    // Make it executable on Unix systems
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o755);
        fs::set_permissions(&current_binary, perms)
            .context("Failed to set executable permissions")?;
    }

    // Cleanup
    fs::remove_file(&archive_path).ok();
    fs::remove_dir_all(&extract_dir).ok();

    info!("✓ Binary updated successfully to {}", release.tag_name);
    println!("✓ dbackup updated to {} successfully!", release.tag_name);
    println!("  Backup of old version saved to: {}", backup_path.display());

    Ok(())
}
