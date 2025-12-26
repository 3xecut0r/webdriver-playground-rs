use std::fs;
use std::io::{self, Cursor};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use anyhow::{Context};

use reqwest::Client;
use serde::Deserialize;
use thirtyfour::prelude::*;
use zip::ZipArchive;


struct DriverProcess {
    child: Child,
}

impl DriverProcess {
    fn start(chromedriver_path: &PathBuf, port: u16) -> io::Result<Self> {
        println!("Starting chromedriver: {}", chromedriver_path.display());
        let child = Command::new(chromedriver_path)
            .args([format!("--port={}", port), "--verbose".to_string()])
            .stdout(Stdio::null()) // can be replaced with inherit() for details
            .stderr(Stdio::null())
            .spawn()?;

        Ok(Self { child })
    }
}

impl Drop for DriverProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

async fn wait_webdriver(port: u16) {
    let url = format!("http://localhost:{port}/status");
    let client = Client::new();

    for _ in 0..50 {
        if let Ok(resp) = client.get(&url).send().await {
            if resp.status().is_success() {
                return ;
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    panic!("chromedriver did not start on port {port}");
}

#[derive(Debug, Deserialize)]
struct LkgWithDownloads {
    channels: Channels,
}

#[derive(Debug, Deserialize)]
struct Channels {
    #[serde(rename = "Stable")]
    stable: Channel,
}

#[derive(Debug, Deserialize)]
struct Channel {
    downloads: Downloads,
}

#[derive(Debug, Deserialize)]
struct Downloads {
    chromedriver: Vec<Download>,
}

#[derive(Debug, Deserialize)]
struct Download {
    platform: String,
    url: String,
}

fn platform_key() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "linux64",
        ("linux", "aarch64") => "linux-arm64",
        ("macos", "x86_64") => "mac-x64",
        ("macos", "aarch64") => "mac-arm64",
        ("windows", "x86_64") => "win64",
        ("windows", "x86") => "win32",
        (os, arch) => panic!("Unsupported platform: {os}/{arch}"),
    }
}

fn driver_filename() -> &'static str {
    if cfg!(target_os = "windows") { "chromedriver.exe" } else { "chromedriver" }
}

fn looks_executable(driver_path: &Path) -> bool {
    Command::new(driver_path)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

async fn ensure_chromedriver(bin_dir: &Path) -> anyhow::Result<PathBuf> {
    fs::create_dir_all(bin_dir).context("create bin_dir")?;
    let driver_path = bin_dir.join(driver_filename());

    if driver_path.exists() {
        if looks_executable(&driver_path) {
            return Ok(driver_path);
        }
        let _ = fs::remove_file(&driver_path);
    }

    println!(
        "Downloading chromedriver for OS={} ARCH={} platform={}",
        std::env::consts::OS,
        std::env::consts::ARCH,
        platform_key()
    );

    let endpoint = "https://googlechromelabs.github.io/chrome-for-testing/last-known-good-versions-with-downloads.json";
    let client = Client::new();
    let meta: LkgWithDownloads = client.get(endpoint).send().await?.json().await?;

    let plat = platform_key();
    let url = meta.channels.stable
        .downloads
        .chromedriver
        .iter()
        .find(|d| d.platform == plat)
        .ok_or_else(|| anyhow::anyhow!("No chromedriver download for platform: {plat}"))?
        .url
        .clone();

    let zip_bytes = client.get(url).send().await?.bytes().await?;
    let mut zip = ZipArchive::new(Cursor::new(zip_bytes))?;

    let entry = format!("chromedriver-{}/{}", plat, driver_filename());
    let mut file = zip.by_name(&entry)
        .with_context(|| format!("Can't find entry in zip: {entry}"))?;

    let tmp_path = bin_dir.join(format!("{}.tmp", driver_filename()));
    {
        let mut out = fs::File::create(&tmp_path).context("create tmp chromedriver file")?;
        io::copy(&mut file, &mut out).context("write tmp chromedriver file")?;
        out.sync_all().ok();
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&tmp_path, fs::Permissions::from_mode(0o755))?;
    }

    fs::rename(&tmp_path, &driver_path).context("rename tmp -> chromedriver")?;

    if !looks_executable(&driver_path) {
        let out = Command::new(&driver_path).arg("--version").output();
        return Err(anyhow::anyhow!(
            "Downloaded chromedriver failed to run.\nResult: {:?}",
            out
        ));
    }

    Ok(driver_path)
}


#[tokio::main]
async fn main() -> WebDriverResult<()> {
    let port = 9515;
    let bin_dir = Path::new("./bin");

    let chromedriver_path = ensure_chromedriver(bin_dir)
        .await
        .expect("failed to ensure chromedriver");

    let _proc = DriverProcess::start(&chromedriver_path, port)
        .expect("failed to start chromedriver");

    wait_webdriver(port).await;

    let caps = DesiredCapabilities::chrome();
    let driver = WebDriver::new(&format!("http://localhost:{port}"), caps).await?;

    driver.goto("https://example.com").await?;
    println!("Title: {}", driver.title().await?);

    driver.quit().await?;
    Ok(())
}
