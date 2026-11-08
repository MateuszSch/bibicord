//! Locating (and if needed downloading) the FFmpeg binary used for
//! hardware encoding/decoding.

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

static FFMPEG: OnceLock<PathBuf> = OnceLock::new();

pub fn path() -> Option<&'static Path> {
    FFMPEG.get().map(|p| p.as_path())
}

pub fn require() -> Result<&'static Path> {
    path().context("FFmpeg nie jest jeszcze gotowy")
}

/// `Command` for ffmpeg that never pops up a console window on Windows.
pub fn command() -> Result<Command> {
    let mut cmd = Command::new(require()?);
    hide_window(&mut cmd);
    cmd.arg("-hide_banner").arg("-nostdin");
    Ok(cmd)
}

pub fn hide_window(cmd: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    #[cfg(not(windows))]
    let _ = cmd;
}

fn exe_name() -> &'static str {
    if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" }
}

fn works(p: &Path) -> bool {
    let mut cmd = Command::new(p);
    hide_window(&mut cmd);
    cmd.arg("-version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn data_dir() -> PathBuf {
    directories::ProjectDirs::from("dev", "bibicord", "BibiCord")
        .map(|d| d.data_local_dir().to_path_buf())
        .unwrap_or_else(|| std::env::temp_dir().join("bibicord"))
}

fn candidates() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Ok(p) = std::env::var("BIBICORD_FFMPEG") {
        v.push(PathBuf::from(p));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            v.push(dir.join(exe_name()));
            // macOS .app bundle: Contents/MacOS/bibicord -> Contents/Resources/ffmpeg
            v.push(dir.join("../Resources").join(exe_name()));
        }
    }
    v.push(data_dir().join(exe_name()));
    v.push(PathBuf::from(exe_name())); // PATH
    if cfg!(target_os = "macos") {
        v.push(PathBuf::from("/opt/homebrew/bin/ffmpeg"));
        v.push(PathBuf::from("/usr/local/bin/ffmpeg"));
    }
    v
}

/// Finds a working ffmpeg or downloads a static build into the app data dir.
/// Blocking; call from a background thread.
pub fn ensure(progress: impl Fn(String)) -> Result<PathBuf> {
    if let Some(p) = path() {
        return Ok(p.to_path_buf());
    }
    for c in candidates() {
        if works(&c) {
            let _ = FFMPEG.set(c.clone());
            return Ok(c);
        }
    }
    use ffmpeg_sidecar::download::*;
    let dir = data_dir();
    std::fs::create_dir_all(&dir)?;
    let url = ffmpeg_download_url()?;
    progress("Pobieranie FFmpeg…".into());
    let archive = download_ffmpeg_package(url, &dir).context("pobieranie FFmpeg")?;
    progress("Rozpakowywanie FFmpeg…".into());
    unpack_ffmpeg(&archive, &dir).context("rozpakowywanie FFmpeg")?;
    let bin = dir.join(exe_name());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755));
    }
    if !works(&bin) {
        bail!("pobrany FFmpeg nie działa");
    }
    let _ = FFMPEG.set(bin.clone());
    Ok(bin)
}

/// Runs ffmpeg with the given args and reports whether it exited cleanly
/// within the timeout. Used to probe hardware encoders.
pub fn probe(args: &[String], timeout: std::time::Duration) -> bool {
    let Ok(mut cmd) = command() else { return false };
    let Ok(mut child) = cmd
        .args(["-loglevel", "error"])
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        return false;
    };
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(s)) => return s.success(),
            Ok(None) if start.elapsed() < timeout => {
                std::thread::sleep(std::time::Duration::from_millis(50))
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return false;
            }
        }
    }
}
