//! Self-update from GitHub Releases (public repo, no token).
//!
//! 1. On start (and every 6 h) ask the GitHub API for the latest release.
//! 2. If its tag is a newer semver than ours, offer it in the UI.
//! 3. On click: download the installer for this platform, verify its
//!    SHA-256 against the published `SHA256SUMS.txt`, install, restart.
//!
//! * Windows: run `BibiCord-Setup.exe /S /RESTART` (NSIS silent mode). The
//!   installer closes the app, replaces the files and starts it again.
//! * macOS: mount the DMG, copy `BibiCord.app` next to the running bundle
//!   (`ditto`), swap it in, relaunch. Files written by the app itself are not
//!   quarantined, so Gatekeeper does not ask again.

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const REPO: &str = "Minfdev/bibicord";
/// Always the newest release (GitHub redirects `latest` to the current tag).
pub const DOWNLOAD_PAGE: &str = "https://github.com/Minfdev/bibicord/releases/latest";
pub const DOWNLOAD_WINDOWS: &str = "https://github.com/Minfdev/bibicord/releases/latest/download/BibiCord-Setup.exe";
pub const DOWNLOAD_MACOS: &str = "https://github.com/Minfdev/bibicord/releases/latest/download/BibiCord.dmg";
pub const CURRENT: &str = env!("CARGO_PKG_VERSION");

/// Installer asset for this platform (fixed names, so
/// `releases/latest/download/<name>` links never change).
pub const ASSET: &str = if cfg!(windows) { "BibiCord-Setup.exe" } else { "BibiCord.dmg" };
const SUMS: &str = "SHA256SUMS.txt";

#[derive(Debug, Clone)]
pub struct Release {
    pub version: semver::Version,
    pub notes: String,
    pub page: String,
    installer: String,
    sums: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub enum State {
    #[default]
    Idle,
    Checking,
    UpToDate,
    Available(Release),
    /// Download progress 0..1.
    Downloading(Release, f32),
    Installing(Release),
    Failed(String),
}

pub type Shared = Arc<Mutex<State>>;

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(10)))
        .timeout_recv_body(Some(Duration::from_secs(120)))
        .user_agent(format!("BibiCord/{CURRENT}"))
        .build()
        .new_agent()
}

/// `v0.3.0` / `0.3.0` -> Version.
pub fn parse_tag(tag: &str) -> Option<semver::Version> {
    semver::Version::parse(tag.trim().trim_start_matches('v')).ok()
}

pub fn is_newer(tag: &str) -> bool {
    match (parse_tag(tag), parse_tag(CURRENT)) {
        (Some(new), Some(cur)) => new > cur,
        _ => false,
    }
}

#[derive(serde::Deserialize)]
struct GhRelease {
    tag_name: String,
    #[serde(default)]
    body: String,
    html_url: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    assets: Vec<GhAsset>,
}

#[derive(serde::Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
}

/// Latest published release, if it is newer than this build.
///
/// `BIBICORD_UPDATE_URL` overrides the API URL (tests: a specific release,
/// e.g. `.../releases/tags/v9.9.9`).
pub fn check() -> Result<Option<Release>> {
    let url = std::env::var("BIBICORD_UPDATE_URL")
        .unwrap_or_else(|_| format!("https://api.github.com/repos/{REPO}/releases/latest"));
    let mut resp = agent()
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .call()
        .context("brak połączenia z GitHubem")?;
    let body = resp.body_mut().with_config().limit(2 << 20).read_to_string()?;
    let r: GhRelease = serde_json::from_str(&body).context("nieczytelna odpowiedź GitHuba")?;
    let test_feed = std::env::var_os("BIBICORD_UPDATE_URL").is_some();
    if r.draft || (r.prerelease && !test_feed) || !is_newer(&r.tag_name) {
        return Ok(None);
    }
    let find = |n: &str| r.assets.iter().find(|a| a.name == n).map(|a| a.browser_download_url.clone());
    let Some(installer) = find(ASSET) else { return Ok(None) };
    Ok(Some(Release {
        version: parse_tag(&r.tag_name).unwrap(),
        notes: r.body.trim().to_string(),
        page: r.html_url,
        installer,
        sums: find(SUMS),
    }))
}

/// Background check that updates `state`.
pub fn spawn_check(state: Shared, ctx: eframe::egui::Context) {
    {
        let mut s = state.lock().unwrap();
        if matches!(*s, State::Downloading(..) | State::Installing(_)) {
            return;
        }
        if !matches!(*s, State::Available(_)) {
            *s = State::Checking;
        }
    }
    std::thread::spawn(move || {
        let next = match check() {
            Ok(Some(r)) => State::Available(r),
            Ok(None) => State::UpToDate,
            Err(e) => {
                tracing::info!("update check: {e:#}");
                State::Idle // stay quiet when offline
            }
        };
        *state.lock().unwrap() = next;
        ctx.request_repaint();
    });
}

/// Downloads, verifies and installs `release`. Restarts the app on success.
pub fn spawn_install(state: Shared, release: Release, ctx: eframe::egui::Context) {
    *state.lock().unwrap() = State::Downloading(release.clone(), 0.0);
    std::thread::spawn(move || {
        let st = state.clone();
        let c = ctx.clone();
        let r = release.clone();
        let progress = move |p: f32| {
            *st.lock().unwrap() = State::Downloading(r.clone(), p);
            c.request_repaint();
        };
        let res = download(&release, progress).and_then(|file| {
            *state.lock().unwrap() = State::Installing(release.clone());
            ctx.request_repaint();
            install(&file)
        });
        if let Err(e) = res {
            tracing::warn!("update: {e:#}");
            *state.lock().unwrap() = State::Failed(format!("{e:#}"));
            ctx.request_repaint();
        }
    });
}

fn download(release: &Release, progress: impl Fn(f32)) -> Result<PathBuf> {
    let dir = crate::ffmpeg::data_dir().join("update");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(ASSET);

    let expected = match &release.sums {
        Some(url) => {
            let text = agent().get(url).call()?.body_mut().read_to_string()?;
            Some(sum_for(&text, ASSET).context("brak sumy kontrolnej instalatora")?)
        }
        None => None,
    };

    let mut resp = agent().get(&release.installer).call().context("pobieranie aktualizacji")?;
    let total = resp.body().content_length().unwrap_or(0);
    let mut reader = resp.body_mut().with_config().limit(1 << 30).reader();
    let mut file = std::fs::File::create(&path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 256 * 1024];
    let mut done = 0u64;
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])?;
        hasher.update(&buf[..n]);
        done += n as u64;
        if total > 0 {
            progress(done as f32 / total as f32);
        }
    }
    file.sync_all()?;
    drop(file);
    let got = hex(&hasher.finalize());
    if let Some(want) = expected {
        if !got.eq_ignore_ascii_case(&want) {
            let _ = std::fs::remove_file(&path);
            bail!("plik aktualizacji jest uszkodzony (zła suma SHA-256)");
        }
    }
    Ok(path)
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Parses `sha256sum` output: `<hex>  <name>` per line.
fn sum_for(text: &str, name: &str) -> Option<String> {
    text.lines().find_map(|l| {
        let mut it = l.split_whitespace();
        let sum = it.next()?;
        let file = it.next()?.trim_start_matches('*');
        (file == name && sum.len() == 64).then(|| sum.to_string())
    })
}

#[cfg(windows)]
fn install(file: &Path) -> Result<()> {
    // Detached: the installer closes this process, then restarts the app.
    let mut cmd = std::process::Command::new(file);
    cmd.args(["/S", "/RESTART"]);
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        cmd.creation_flags(DETACHED_PROCESS);
    }
    cmd.spawn().context("nie udało się uruchomić instalatora")?;
    // The installer kills us; exit on our own if it does not.
    std::thread::sleep(Duration::from_secs(20));
    std::process::exit(0);
}

#[cfg(target_os = "macos")]
fn install(file: &Path) -> Result<()> {
    use std::process::Command;
    let bundle = std::env::current_exe()?
        .ancestors()
        .find(|p| p.extension().is_some_and(|e| e == "app"))
        .map(Path::to_path_buf)
        .context("aplikacja nie działa z paczki .app – zainstaluj ręcznie z DMG")?;
    let parent = bundle.parent().context("brak folderu aplikacji")?;
    let mnt = file.with_extension("mnt");
    let _ = std::fs::create_dir_all(&mnt);
    let ok = Command::new("hdiutil")
        .args(["attach", "-nobrowse", "-readonly", "-quiet", "-mountpoint"])
        .arg(&mnt)
        .arg(file)
        .status()?
        .success();
    if !ok {
        bail!("nie udało się otworzyć pliku DMG");
    }
    let staged = parent.join(".BibiCord-update.app");
    let _ = std::fs::remove_dir_all(&staged);
    let copied = Command::new("ditto").arg(mnt.join("BibiCord.app")).arg(&staged).status();
    let _ = Command::new("hdiutil").args(["detach", "-quiet", "-force"]).arg(&mnt).status();
    if !copied?.success() {
        let _ = std::fs::remove_dir_all(&staged);
        bail!("brak uprawnień do zapisu w {} – zainstaluj ręcznie z DMG", parent.display());
    }
    // Swap: old bundle aside, new one in place (both renames are atomic).
    let old = parent.join(".BibiCord-old.app");
    let _ = std::fs::remove_dir_all(&old);
    std::fs::rename(&bundle, &old).context("podmiana aplikacji")?;
    if let Err(e) = std::fs::rename(&staged, &bundle) {
        let _ = std::fs::rename(&old, &bundle);
        return Err(e).context("podmiana aplikacji");
    }
    let _ = std::fs::remove_dir_all(&old);
    let _ = std::fs::remove_file(file);
    // Relaunch once this process has exited.
    Command::new("/bin/sh")
        .arg("-c")
        .arg(format!("sleep 1; /usr/bin/open -n '{}'", bundle.display().to_string().replace('\'', r"'\''")))
        .spawn()?;
    std::process::exit(0);
}

#[cfg(not(any(windows, target_os = "macos")))]
fn install(_file: &Path) -> Result<()> {
    bail!("automatyczna aktualizacja nie jest dostępna na tym systemie")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_and_sums() {
        assert!(is_newer("v999.0.0"));
        assert!(!is_newer(&format!("v{CURRENT}")));
        assert!(!is_newer("v0.0.1"));
        assert!(!is_newer("nonsense"));
        let sums = "abc  other.dmg\n".to_string() + &"a".repeat(64) + "  BibiCord.dmg\n" + &"b".repeat(64) + " *BibiCord-Setup.exe\n";
        assert_eq!(sum_for(&sums, "BibiCord.dmg"), Some("a".repeat(64)));
        assert_eq!(sum_for(&sums, "BibiCord-Setup.exe"), Some("b".repeat(64)));
        assert_eq!(sum_for(&sums, "x"), None);
    }

    /// Talks to the real GitHub API. Run with `cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn check_live() {
        println!("{:?}", check());
    }
}
