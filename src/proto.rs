//! Wire protocol shared by all peers.

use anyhow::{Context, Result, bail};
use iroh::{EndpointAddr, EndpointId};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub const ALPN: &[u8] = b"bibicord/2";

/// First byte of every bidirectional stream.
pub const TAG_CONTROL: u8 = 0;
pub const TAG_WATCH: u8 = 1;

/// First byte of every datagram.
pub const DGRAM_VOICE: u8 = 1;

const MAX_FRAME: usize = 1 << 20;
const TICKET_PREFIX: &str = "bibi";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Codec {
    H264,
    Hevc,
}

impl Codec {
    pub fn label(self) -> &'static str {
        match self {
            Codec::H264 => "H.264",
            Codec::Hevc => "HEVC",
        }
    }

    /// Raw Annex B demuxer name in ffmpeg.
    pub fn ffmpeg_format(self) -> &'static str {
        match self {
            Codec::H264 => "h264",
            Codec::Hevc => "hevc",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamInfo {
    pub codec: Codec,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub bitrate_kbps: u32,
    pub encoder: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Msg {
    Hello {
        name: String,
        room: String,
        peers: Vec<EndpointId>,
        stream: Option<StreamInfo>,
        voice: bool,
        muted: bool,
        deafened: bool,
    },
    Chat {
        text: String,
    },
    Stream(Option<StreamInfo>),
    Voice(bool),
    Mute {
        muted: bool,
        deafened: bool,
    },
}

pub async fn write_msg(w: &mut (impl AsyncWrite + Unpin), msg: &Msg) -> Result<()> {
    let bytes = postcard::to_stdvec(msg)?;
    w.write_all(&(bytes.len() as u32).to_le_bytes()).await?;
    w.write_all(&bytes).await?;
    Ok(())
}

pub async fn read_msg(r: &mut (impl AsyncRead + Unpin)) -> Result<Msg> {
    let mut len = [0u8; 4];
    r.read_exact(&mut len).await?;
    let len = u32::from_le_bytes(len) as usize;
    if len > MAX_FRAME {
        bail!("frame too large: {len}");
    }
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf).await?;
    Ok(postcard::from_bytes(&buf)?)
}

/// A room invite is just the address of any member, encoded as text.
pub fn encode_ticket(addr: &EndpointAddr) -> String {
    let bytes = postcard::to_stdvec(addr).expect("serialize addr");
    format!(
        "{TICKET_PREFIX}{}",
        data_encoding::BASE32_NOPAD.encode(&bytes).to_ascii_lowercase()
    )
}

pub fn decode_ticket(ticket: &str) -> Result<EndpointAddr> {
    let t = ticket.trim();
    let body = t
        .strip_prefix(TICKET_PREFIX)
        .context("to nie jest zaproszenie BibiCord")?;
    let bytes = data_encoding::BASE32_NOPAD
        .decode(body.to_ascii_uppercase().as_bytes())
        .context("uszkodzone zaproszenie")?;
    Ok(postcard::from_bytes(&bytes).context("uszkodzone zaproszenie")?)
}
