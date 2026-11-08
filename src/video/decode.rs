//! Hardware decoding of a received bitstream into RGBA frames for the UI.

use crate::ffmpeg;
use crate::proto::Codec;
use anyhow::{Context, Result};
use bytes::Bytes;
use ffmpeg_sidecar::command::FfmpegCommand;
use ffmpeg_sidecar::event::{FfmpegEvent, LogLevel};
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

pub struct Frame {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
}

#[derive(Default)]
pub struct FrameSlot {
    latest: Mutex<Option<Frame>>,
    count: AtomicU64,
}

impl FrameSlot {
    pub fn take(&self) -> Option<Frame> {
        self.latest.lock().unwrap().take()
    }

    pub fn count(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }
}

pub struct Decoder {
    tx: mpsc::Sender<Bytes>,
    child: Arc<Mutex<ffmpeg_sidecar::child::FfmpegChild>>,
}

impl Drop for Decoder {
    fn drop(&mut self) {
        if let Ok(mut c) = self.child.lock() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

impl Decoder {
    pub fn start(codec: Codec, slot: Arc<FrameSlot>, repaint: impl Fn() + Send + 'static) -> Result<Self> {
        let mut child = FfmpegCommand::new_with_path(ffmpeg::require()?)
            .hide_banner()
            // Hardware decode, no frame threading (= no added frames of
            // latency). Note: `-fflags nobuffer` must NOT be used, it drops
            // the probed packets, i.e. the keyframe we start with.
            .args(["-hwaccel", "auto", "-threads", "1", "-flags", "low_delay"])
            .args(["-probesize", "32", "-analyzeduration", "0"])
            .args(["-f", codec.ffmpeg_format(), "-i", "pipe:0"])
            .args(["-an", "-fps_mode", "passthrough"])
            .args(["-f", "rawvideo", "-pix_fmt", "rgba", "pipe:1"])
            .spawn()
            .context("nie można uruchomić dekodera")?;
        let mut stdin = child.take_stdin().context("brak stdin dekodera")?;
        let iter = child.iter()?;

        let (tx, mut rx) = mpsc::channel::<Bytes>(64);
        std::thread::spawn(move || {
            while let Some(b) = rx.blocking_recv() {
                if stdin.write_all(&b).is_err() {
                    break;
                }
            }
        });
        std::thread::spawn(move || {
            for ev in iter {
                match ev {
                    FfmpegEvent::OutputFrame(f) => {
                        slot.count.fetch_add(1, Ordering::Relaxed);
                        *slot.latest.lock().unwrap() = Some(Frame {
                            width: f.width as usize,
                            height: f.height as usize,
                            rgba: f.data,
                        });
                        repaint();
                    }
                    FfmpegEvent::Log(LogLevel::Error | LogLevel::Fatal, l) => {
                        tracing::warn!("ffmpeg dec: {l}")
                    }
                    FfmpegEvent::Error(e) => tracing::warn!("ffmpeg dec: {e}"),
                    _ => {}
                }
            }
        });
        Ok(Decoder { tx, child: Arc::new(Mutex::new(child)) })
    }

    /// Feeds received bitstream. Applies backpressure if decoding lags.
    pub async fn feed(&self, data: Bytes) -> Result<()> {
        self.tx.send(data).await.ok().context("dekoder zamknięty")
    }
}
