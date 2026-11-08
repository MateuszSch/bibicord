//! Voice chat: microphone -> Opus (48 kHz mono, 20 ms) -> QUIC datagrams,
//! and per-peer Opus decode + jitter buffer + mixing on playback.

use anyhow::{Context, Result, anyhow};
use bytes::Bytes;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use iroh::EndpointId;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

const RATE: u32 = 48_000;
const FRAME: usize = 960; // 20 ms
const PREBUFFER_MS: usize = 40;
const MAX_BUFFER_MS: usize = 200;
const SPEAK_THRESHOLD: f32 = 0.015;

pub struct Voice {
    shared: Arc<Shared>,
    stop: Arc<AtomicBool>,
}

struct Shared {
    muted: AtomicBool,
    deafened: AtomicBool,
    out_rate: Mutex<u32>,
    peers: Mutex<HashMap<EndpointId, PeerAudio>>,
    me_speaking_until: Mutex<Instant>,
}

struct PeerAudio {
    dec: opus::Decoder,
    resample: Linear,
    queue: VecDeque<f32>,
    playing: bool,
    last_seq: Option<u16>,
    speaking_until: Instant,
}

impl Drop for Voice {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

impl Voice {
    /// Starts capture + playback. Encoded packets (with sequence number
    /// prefix) are sent to `packets`.
    pub fn start(
        packets: mpsc::UnboundedSender<Bytes>,
        devices: Devices,
        muted: bool,
        deafened: bool,
    ) -> Result<Self> {
        let shared = Arc::new(Shared {
            muted: AtomicBool::new(muted),
            deafened: AtomicBool::new(deafened),
            out_rate: Mutex::new(RATE),
            peers: Mutex::new(HashMap::new()),
            me_speaking_until: Mutex::new(Instant::now()),
        });
        let stop = Arc::new(AtomicBool::new(false));
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        {
            let shared = shared.clone();
            let stop = stop.clone();
            // cpal streams are not Send on every platform: own them on a
            // dedicated thread for their whole life.
            std::thread::spawn(move || {
                if fake_audio() {
                    let _ = ready_tx.send(Ok(()));
                    run_fake(shared, packets, stop);
                    return;
                }
                match run_audio(shared, packets, &devices) {
                    Ok((a, b)) => {
                        let _ = ready_tx.send(Ok(()));
                        while !stop.load(Ordering::SeqCst) {
                            std::thread::sleep(Duration::from_millis(100));
                        }
                        drop((a, b));
                    }
                    Err(e) => {
                        let _ = ready_tx.send(Err(e));
                    }
                }
            });
        }
        ready_rx
            .recv_timeout(Duration::from_secs(10))
            .map_err(|_| anyhow!("urządzenie audio nie odpowiada"))??;
        Ok(Voice { shared, stop })
    }

    pub fn set_muted(&self, m: bool) {
        self.shared.muted.store(m, Ordering::Relaxed);
    }

    pub fn set_deafened(&self, d: bool) {
        self.shared.deafened.store(d, Ordering::Relaxed);
    }

    pub fn me_speaking(&self) -> bool {
        *self.shared.me_speaking_until.lock().unwrap() > Instant::now()
    }

    pub fn speaking(&self) -> Vec<EndpointId> {
        let now = Instant::now();
        self.shared
            .peers
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, p)| p.speaking_until > now)
            .map(|(id, _)| *id)
            .collect()
    }

    pub fn remove_peer(&self, id: &EndpointId) {
        self.shared.peers.lock().unwrap().remove(id);
    }

    /// Handles one received voice datagram (without the type byte).
    pub fn receive(&self, from: EndpointId, data: &[u8]) {
        if data.len() < 3 {
            return;
        }
        let seq = u16::from_le_bytes([data[0], data[1]]);
        let payload = &data[2..];
        let out_rate = *self.shared.out_rate.lock().unwrap();
        let mut peers = self.shared.peers.lock().unwrap();
        let p = match peers.entry(from) {
            std::collections::hash_map::Entry::Occupied(e) => e.into_mut(),
            std::collections::hash_map::Entry::Vacant(e) => {
                let Ok(dec) = opus::Decoder::new(RATE, opus::Channels::Mono) else { return };
                e.insert(PeerAudio {
                    dec,
                    resample: Linear::new(RATE, out_rate),
                    queue: VecDeque::new(),
                    playing: false,
                    last_seq: None,
                    speaking_until: Instant::now(),
                })
            }
        };
        let mut pcm = [0f32; FRAME * 6];
        if let Some(last) = p.last_seq {
            let gap = seq.wrapping_sub(last);
            if gap == 0 || gap > 0x8000 {
                return; // duplicate or reordered-late packet
            }
            // Conceal up to 3 lost frames.
            for _ in 1..gap.min(4) {
                if let Ok(n) = p.dec.decode_float(&[], &mut pcm, false) {
                    let mut tmp = Vec::new();
                    p.resample.process(&pcm[..n], &mut tmp);
                    p.queue.extend(tmp);
                }
            }
        }
        p.last_seq = Some(seq);
        let Ok(n) = p.dec.decode_float(payload, &mut pcm, false) else { return };
        if rms(&pcm[..n]) > SPEAK_THRESHOLD {
            p.speaking_until = Instant::now() + Duration::from_millis(300);
        }
        let mut tmp = Vec::with_capacity(n * 2);
        p.resample.process(&pcm[..n], &mut tmp);
        p.queue.extend(tmp);
        let max = out_rate as usize * MAX_BUFFER_MS / 1000;
        if p.queue.len() > max {
            let drop = p.queue.len() - max / 2;
            p.queue.drain(..drop);
        }
    }
}

fn rms(s: &[f32]) -> f32 {
    if s.is_empty() {
        return 0.0;
    }
    (s.iter().map(|x| x * x).sum::<f32>() / s.len() as f32).sqrt()
}

/// Selected audio devices (cpal device ids); `None` = system default.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Devices {
    pub input: Option<String>,
    pub output: Option<String>,
}

/// One selectable audio device.
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
}

/// Lists (inputs, outputs).
pub fn list_devices() -> (Vec<DeviceInfo>, Vec<DeviceInfo>) {
    let host = cpal::default_host();
    let info = |d: cpal::Device| {
        Some(DeviceInfo {
            id: d.id().ok()?.to_string(),
            name: d.description().ok()?.name().to_string(),
        })
    };
    let ins = host
        .input_devices()
        .map(|it| it.filter_map(info).collect())
        .unwrap_or_default();
    let outs = host
        .output_devices()
        .map(|it| it.filter_map(info).collect())
        .unwrap_or_default();
    (ins, outs)
}

fn fake_audio() -> bool {
    std::env::var_os("BIBICORD_FAKE_AUDIO").is_some()
}

/// Encoder side: takes mono samples at `in_rate`, sends Opus packets.
fn capture_pipeline(
    shared: Arc<Shared>,
    packets: mpsc::UnboundedSender<Bytes>,
    in_rate: u32,
) -> Result<impl FnMut(&[f32]) + Send + 'static> {
    let mut enc = opus::Encoder::new(RATE, opus::Channels::Mono, opus::Application::Voip)?;
    enc.set_bitrate(opus::Bitrate::Bits(48_000))?;
    enc.set_inband_fec(true)?;
    let mut resample = Linear::new(in_rate, RATE);
    let mut pending: Vec<f32> = Vec::with_capacity(FRAME * 4);
    let mut seq: u16 = 0;
    let mut silent_frames = 0u32;
    Ok(move |mono: &[f32]| {
        resample.process(mono, &mut pending);
        while pending.len() >= FRAME {
            let frame: Vec<f32> = pending.drain(..FRAME).collect();
            if shared.muted.load(Ordering::Relaxed) {
                continue;
            }
            // Simple gate: keep sending 300 ms after speech, then go silent.
            if rms(&frame) > SPEAK_THRESHOLD {
                silent_frames = 0;
                *shared.me_speaking_until.lock().unwrap() =
                    Instant::now() + Duration::from_millis(300);
            } else {
                silent_frames += 1;
            }
            if silent_frames > 15 {
                continue;
            }
            let mut out = [0u8; 1500];
            out[0..2].copy_from_slice(&seq.to_le_bytes());
            if let Ok(n) = enc.encode_float(&frame, &mut out[2..]) {
                let _ = packets.send(Bytes::copy_from_slice(&out[..n + 2]));
            }
            seq = seq.wrapping_add(1);
        }
    })
}

/// Playback side: mixes all peers into `frames` mono samples.
fn mixer(shared: Arc<Shared>, out_rate: u32) -> impl FnMut(usize) -> Vec<f32> + Send + 'static {
    *shared.out_rate.lock().unwrap() = out_rate;
    let prebuffer = out_rate as usize * PREBUFFER_MS / 1000;
    move |frames: usize| -> Vec<f32> {
        let mut out = vec![0f32; frames];
        let deaf = shared.deafened.load(Ordering::Relaxed);
        let mut peers = shared.peers.lock().unwrap();
        for p in peers.values_mut() {
            if !p.playing {
                if p.queue.len() >= prebuffer {
                    p.playing = true;
                } else {
                    continue;
                }
            }
            for o in out.iter_mut() {
                match p.queue.pop_front() {
                    Some(s) => *o += s,
                    None => {
                        p.playing = false;
                        break;
                    }
                }
            }
        }
        if deaf {
            out.iter_mut().for_each(|s| *s = 0.0);
        }
        out.iter_mut().for_each(|s| *s = s.clamp(-1.0, 1.0));
        out
    }
}

/// Test mode without audio devices. With `BIBICORD_FAKE_TONE` the
/// "microphone" plays a tone that alternates between speech and silence.
fn run_fake(shared: Arc<Shared>, packets: mpsc::UnboundedSender<Bytes>, stop: Arc<AtomicBool>) {
    let Ok(mut capture) = capture_pipeline(shared.clone(), packets, RATE) else { return };
    let mut mix = mixer(shared, RATE);
    let tone = std::env::var_os("BIBICORD_FAKE_TONE").is_some();
    let start = Instant::now();
    let mut n: u64 = 0;
    while !stop.load(Ordering::SeqCst) {
        let t = start.elapsed().as_secs_f32();
        let speaking = tone && (t % 3.5) < 2.0;
        let frame: Vec<f32> = (0..FRAME)
            .map(|i| {
                if speaking {
                    let x = (n * FRAME as u64 + i as u64) as f32 / RATE as f32;
                    0.2 * (x * 2.0 * std::f32::consts::PI * 220.0).sin()
                } else {
                    0.0
                }
            })
            .collect();
        capture(&frame);
        let _ = mix(FRAME);
        n += 1;
        let next = start + Duration::from_millis(20 * n);
        if let Some(d) = next.checked_duration_since(Instant::now()) {
            std::thread::sleep(d);
        }
    }
}

fn find_device(host: &cpal::Host, id: Option<&str>, input: bool) -> Option<cpal::Device> {
    let id = id?.parse().ok()?;
    let d = host.device_by_id(&id)?;
    let ok = if input { d.default_input_config().is_ok() } else { d.default_output_config().is_ok() };
    ok.then_some(d)
}

fn run_audio(
    shared: Arc<Shared>,
    packets: mpsc::UnboundedSender<Bytes>,
    devices: &Devices,
) -> Result<(cpal::Stream, cpal::Stream)> {
    let host = cpal::default_host();
    let input = find_device(&host, devices.input.as_deref(), true)
        .or_else(|| host.default_input_device())
        .context("brak mikrofonu")?;
    let output = find_device(&host, devices.output.as_deref(), false)
        .or_else(|| host.default_output_device())
        .context("brak głośników")?;
    let in_cfg = input.default_input_config()?;
    let out_cfg = output.default_output_config()?;

    // ---- capture ----
    let in_ch = in_cfg.channels() as usize;
    let mut on_input = capture_pipeline(shared.clone(), packets, in_cfg.sample_rate())?;
    let err = |e| tracing::warn!("audio: {e}");
    let in_stream = match in_cfg.sample_format() {
        cpal::SampleFormat::F32 => input.build_input_stream::<f32, _, _>(
            in_cfg.config(),
            move |d: &[f32], _| on_input(&downmix(d, in_ch, |x| x)),
            err,
            None,
        )?,
        cpal::SampleFormat::I16 => input.build_input_stream::<i16, _, _>(
            in_cfg.config(),
            move |d: &[i16], _| on_input(&downmix(d, in_ch, |x| x as f32 / 32768.0)),
            err,
            None,
        )?,
        f => anyhow::bail!("nieobsługiwany format mikrofonu {f:?}"),
    };

    // ---- playback ----
    let out_ch = out_cfg.channels() as usize;
    let mut mix = mixer(shared.clone(), out_cfg.sample_rate());
    let out_stream = match out_cfg.sample_format() {
        cpal::SampleFormat::F32 => output.build_output_stream::<f32, _, _>(
            out_cfg.config(),
            move |d: &mut [f32], _| {
                let m = mix(d.len() / out_ch);
                for (f, s) in d.chunks_mut(out_ch).zip(m) {
                    f.fill(s);
                }
            },
            err,
            None,
        )?,
        cpal::SampleFormat::I16 => output.build_output_stream::<i16, _, _>(
            out_cfg.config(),
            move |d: &mut [i16], _| {
                let m = mix(d.len() / out_ch);
                for (f, s) in d.chunks_mut(out_ch).zip(m) {
                    f.fill((s * 32767.0) as i16);
                }
            },
            err,
            None,
        )?,
        f => anyhow::bail!("nieobsługiwany format głośników {f:?}"),
    };
    in_stream.play()?;
    out_stream.play()?;
    Ok((in_stream, out_stream))
}

fn downmix<T: Copy>(d: &[T], ch: usize, conv: impl Fn(T) -> f32) -> Vec<f32> {
    if ch <= 1 {
        return d.iter().map(|&x| conv(x)).collect();
    }
    d.chunks(ch)
        .map(|f| f.iter().map(|&x| conv(x)).sum::<f32>() / ch as f32)
        .collect()
}

/// Streaming linear resampler.
struct Linear {
    step: f64,
    t: f64,
    prev: f32,
}

impl Linear {
    fn new(from: u32, to: u32) -> Self {
        Linear { step: from as f64 / to as f64, t: 0.0, prev: 0.0 }
    }

    fn process(&mut self, input: &[f32], out: &mut Vec<f32>) {
        if input.is_empty() {
            return;
        }
        if self.step == 1.0 {
            out.extend_from_slice(input);
            return;
        }
        // Sample positions: -1 is `prev`, 0.. are `input`.
        let last = input.len() as f64 - 1.0;
        while self.t < last {
            let i = self.t.floor();
            let frac = (self.t - i) as f32;
            let i = i as isize;
            let a = if i < 0 { self.prev } else { input[i as usize] };
            let b = input[(i + 1) as usize];
            out.push(a + (b - a) * frac);
            self.t += self.step;
        }
        self.t -= input.len() as f64;
        self.prev = input[input.len() - 1];
    }
}

#[cfg(test)]
mod tests {
    use super::Linear;

    #[test]
    fn resampler_keeps_rate() {
        let mut r = Linear::new(44_100, 48_000);
        let mut out = Vec::new();
        for _ in 0..100 {
            r.process(&[0.5; 441], &mut out);
        }
        assert!((out.len() as i64 - 48_000).abs() < 3, "{}", out.len());
        assert!(out[10..].iter().all(|s| (s - 0.5).abs() < 1e-6));
    }
}
