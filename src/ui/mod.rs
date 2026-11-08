//! BibiCord UI. Design system: `theme` (tokens) + `widgets` (components).
//! Screens: `start`, then the room (`sidebar` + `chat` | `stage`).

mod chat;
mod dialogs;
mod sidebar;
mod stage;
mod start;
pub mod theme;
mod widgets;

use crate::net::{Event, Events, MuteState, Node};
use crate::proto::StreamInfo;
use crate::video::encode::Settings;
use eframe::egui::{self, Key, TextureHandle, ViewportCommand};
use iroh::EndpointId;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use theme::c;

pub(crate) struct PeerView {
    pub id: EndpointId,
    pub name: String,
    pub stream: Option<StreamInfo>,
    pub voice: bool,
    pub mute: MuteState,
    pub color: egui::Color32,
}

pub(crate) enum ChatKind {
    Message { from: Option<EndpointId>, name: String, text: String },
    System { icon: &'static str, who: String, text: &'static str, good: bool },
}

pub(crate) struct ChatItem {
    pub kind: ChatKind,
    pub time: chrono::DateTime<chrono::Local>,
}

#[derive(PartialEq, Clone, Copy)]
pub(crate) enum Screen {
    Start,
    Room,
}

#[derive(PartialEq, Clone, Copy)]
pub(crate) enum View {
    Chat,
    Voice,
}

#[derive(PartialEq, Clone, Copy)]
pub(crate) enum Dialog {
    GoLive,
    Settings,
    Invite,
    Update,
}

pub struct App {
    pub(crate) node: Node,
    events: std::sync::mpsc::Receiver<Event>,
    pub(crate) screen: Screen,
    pub(crate) view: View,
    pub(crate) dialog: Option<Dialog>,
    pub(crate) net_ready: bool,
    pub(crate) ffmpeg_status: Arc<Mutex<String>>,

    pub(crate) name: String,
    pub(crate) room_input: String,
    pub(crate) ticket_input: String,
    pub(crate) join_error: Option<String>,
    pub(crate) ticket: Option<String>,
    pub(crate) copied_at: Option<Instant>,
    pub(crate) room: String,
    pub(crate) my_id: Option<EndpointId>,

    pub(crate) peers: Vec<PeerView>,
    pub(crate) chat: Vec<ChatItem>,
    pub(crate) chat_input: String,
    pub(crate) unread: usize,
    pub(crate) stage_chat: bool,

    pub(crate) my_stream: Option<StreamInfo>,
    pub(crate) stream_starting: bool,
    pub(crate) settings: Settings,
    pub(crate) displays: Vec<String>,
    pub(crate) audio_devices: Option<(Vec<crate::voice::DeviceInfo>, Vec<crate::voice::DeviceInfo>)>,

    pub(crate) my_voice: bool,
    pub(crate) voice_starting: bool,
    pub(crate) mute: MuteState,

    pub(crate) watching: Option<EndpointId>,
    pub(crate) texture: Option<TextureHandle>,
    pub(crate) video_size: [usize; 2],
    pub(crate) fps: (Instant, u64, f32),
    pub(crate) fullscreen: bool,

    pub(crate) status: String,
    pub(crate) error: Option<(String, Instant)>,
    pub(crate) update: crate::update::Shared,
    pub(crate) sounds: crate::sounds::Player,
    update_checked: Option<Instant>,
    demo: Option<crate::demo::Demo>,
}

fn name_file() -> std::path::PathBuf {
    crate::ffmpeg::data_dir().join("name.txt")
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, rt: tokio::runtime::Handle) -> Self {
        theme::install(&cc.egui_ctx);
        let (tx, rx) = std::sync::mpsc::channel();
        let events = Events::new(tx, cc.egui_ctx.clone());
        let node = Node::new(rt, events);

        let ffmpeg_status = Arc::new(Mutex::new("Przygotowywanie FFmpeg…".to_string()));
        {
            let st = ffmpeg_status.clone();
            let ctx = cc.egui_ctx.clone();
            std::thread::spawn(move || {
                let set = |s: String| {
                    *st.lock().unwrap() = s;
                    ctx.request_repaint();
                };
                match crate::ffmpeg::ensure(&set) {
                    Ok(_) => set(String::new()),
                    Err(e) => set(format!("FFmpeg niedostępny: {e:#}")),
                }
            });
        }

        let name = std::fs::read_to_string(name_file())
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        App {
            node,
            events: rx,
            screen: Screen::Start,
            view: View::Chat,
            dialog: None,
            net_ready: false,
            ffmpeg_status,
            name,
            room_input: String::new(),
            ticket_input: String::new(),
            join_error: None,
            ticket: None,
            copied_at: None,
            room: String::new(),
            my_id: None,
            peers: Vec::new(),
            chat: Vec::new(),
            chat_input: String::new(),
            unread: 0,
            stage_chat: false,
            my_stream: None,
            stream_starting: false,
            settings: Settings::default(),
            displays: Vec::new(),
            audio_devices: None,
            my_voice: false,
            voice_starting: false,
            mute: MuteState::default(),
            watching: None,
            texture: None,
            video_size: [0, 0],
            fps: (Instant::now(), 0, 0.0),
            fullscreen: false,
            status: String::new(),
            error: None,
            update: Default::default(),
            sounds: crate::sounds::Player::new(),
            update_checked: None,
            demo: crate::demo::Demo::from_env(),
        }
    }

    pub(crate) fn peer(&self, id: &EndpointId) -> Option<&PeerView> {
        self.peers.iter().find(|p| p.id == *id)
    }

    pub(crate) fn my_color(&self) -> egui::Color32 {
        match self.my_id {
            Some(id) => theme::avatar_color(id.as_bytes()),
            None => theme::avatar_color(self.name.as_bytes()),
        }
    }

    fn system(&mut self, icon: &'static str, who: String, text: &'static str, good: bool) {
        self.chat.push(ChatItem {
            kind: ChatKind::System { icon, who, text, good },
            time: chrono::Local::now(),
        });
    }

    /// Plays a UI sound on the selected output device.
    pub(crate) fn cue(&self, cue: crate::sounds::Cue) {
        self.sounds.play(cue, self.node.devices().output);
    }

    fn chat_visible(&self) -> bool {
        self.view == View::Chat || self.stage_chat
    }

    fn handle_events(&mut self, ctx: &egui::Context) {
        use crate::sounds::Cue;
        use widgets::ic;
        while let Ok(ev) = self.events.try_recv() {
            match ev {
                Event::Ready => {
                    self.net_ready = true;
                    self.my_id = self.node.my_id();
                }
                Event::Ticket(t) => self.ticket = Some(t),
                Event::RoomName(r) => {
                    self.room = r;
                    ctx.send_viewport_cmd(ViewportCommand::Title(format!("{} – BibiCord", self.room)));
                }
                Event::PeerJoined { id, name } => {
                    if let Some(p) = self.peers.iter_mut().find(|p| p.id == id) {
                        p.name = name;
                    } else {
                        self.status.clear();
                        self.system(ic::ARROW_RIGHT, name.clone(), "dołącza do pokoju.", true);
                        self.peers.push(PeerView {
                            id,
                            name,
                            stream: None,
                            voice: false,
                            mute: MuteState::default(),
                            color: theme::avatar_color(id.as_bytes()),
                        });
                    }
                }
                Event::PeerLeft { id } => {
                    if let Some(p) = self.peer(&id) {
                        if p.voice && self.my_voice {
                            self.cue(Cue::PeerLeave);
                        }
                        let n = p.name.clone();
                        self.system(ic::ARROW_LEFT, n, "opuszcza pokój.", false);
                    }
                    self.peers.retain(|p| p.id != id);
                }
                Event::PeerStream { id, info } => {
                    let mut started = None;
                    let mut stopped = false;
                    if let Some(p) = self.peers.iter_mut().find(|p| p.id == id) {
                        if p.stream.is_none() && info.is_some() {
                            started = Some(p.name.clone());
                        }
                        stopped = p.stream.is_some() && info.is_none();
                        p.stream = info;
                    }
                    if let Some(n) = started {
                        self.cue(Cue::StreamStart);
                        self.system(ic::BROADCAST, n, "udostępnia ekran.", true);
                    }
                    if stopped {
                        self.cue(Cue::StreamStop);
                    }
                }
                Event::PeerVoice { id, on } => {
                    let mut changed = false;
                    if let Some(p) = self.peers.iter_mut().find(|p| p.id == id) {
                        changed = p.voice != on;
                        p.voice = on;
                    }
                    if changed && self.my_voice {
                        self.cue(if on { Cue::PeerJoin } else { Cue::PeerLeave });
                    }
                }
                Event::PeerMute { id, muted, deafened } => {
                    if let Some(p) = self.peers.iter_mut().find(|p| p.id == id) {
                        p.mute = MuteState { muted, deafened };
                    }
                }
                Event::Chat { from, name, text } => {
                    if from.is_some() && !self.chat_visible() {
                        self.unread += 1;
                    }
                    if from.is_some() && (!self.chat_visible() || !ctx.input(|i| i.focused)) {
                        self.cue(Cue::Message);
                    }
                    self.chat.push(ChatItem {
                        kind: ChatKind::Message { from, name, text },
                        time: chrono::Local::now(),
                    })
                }
                Event::MyStream(info) => {
                    self.stream_starting = false;
                    if self.my_stream.is_some() != info.is_some() {
                        self.cue(if info.is_some() { Cue::StreamStart } else { Cue::StreamStop });
                    }
                    self.my_stream = info;
                }
                Event::MyVoice(on) => {
                    self.voice_starting = false;
                    // Device changes reconnect without a "false" in between.
                    if self.my_voice != on {
                        self.cue(if on { Cue::Connect } else { Cue::Disconnect });
                    }
                    self.my_voice = on;
                }
                Event::Watching(w) => {
                    self.watching = w;
                    if w.is_none() {
                        self.texture = None;
                        self.video_size = [0, 0];
                    }
                }
                Event::Status(s) => self.status = s,
                Event::Error(e) => {
                    self.stream_starting = false;
                    self.voice_starting = false;
                    self.status.clear();
                    self.error = Some((e, Instant::now()));
                }
            }
        }
    }

    pub(crate) fn enter_room(&mut self) {
        let _ = std::fs::create_dir_all(crate::ffmpeg::data_dir());
        let _ = std::fs::write(name_file(), &self.name);
        self.screen = Screen::Room;
        self.view = View::Chat;
        self.my_id = self.node.my_id();
    }

    pub(crate) fn leave_room(&mut self, ctx: &egui::Context) {
        if self.my_voice {
            self.cue(crate::sounds::Cue::Disconnect);
        }
        self.node.leave();
        self.screen = Screen::Start;
        self.peers.clear();
        self.chat.clear();
        self.unread = 0;
        self.ticket = None;
        self.room.clear();
        self.my_stream = None;
        self.my_voice = false;
        self.watching = None;
        self.texture = None;
        self.dialog = None;
        self.status.clear();
        ctx.send_viewport_cmd(ViewportCommand::Title("BibiCord".into()));
        if self.fullscreen {
            self.fullscreen = false;
            ctx.send_viewport_cmd(ViewportCommand::Fullscreen(false));
        }
    }

    pub(crate) fn join_voice(&mut self) {
        if !self.my_voice && !self.voice_starting {
            self.voice_starting = true;
            self.node.join_voice();
        }
    }

    pub(crate) fn set_mute(&mut self, mute: MuteState) {
        use crate::sounds::Cue;
        let old = self.mute;
        if mute.deafened != old.deafened {
            self.cue(if mute.deafened { Cue::Deafen } else { Cue::Undeafen });
        } else if mute.muted != old.muted {
            self.cue(if mute.muted { Cue::Mute } else { Cue::Unmute });
        }
        self.mute = mute;
        self.node.set_mute(mute);
    }

    pub(crate) fn toggle_mic(&mut self) {
        let mut m = self.mute;
        if m.deafened {
            // Unmuting while deafened undeafens too (as users expect).
            m.deafened = false;
            m.muted = false;
        } else {
            m.muted = !m.muted;
        }
        self.set_mute(m);
    }

    pub(crate) fn toggle_deafen(&mut self) {
        let mut m = self.mute;
        m.deafened = !m.deafened;
        self.set_mute(m);
    }

    pub(crate) fn open_go_live(&mut self) {
        self.displays = crate::video::encode::displays();
        if self.settings.display >= self.displays.len() {
            self.settings.display = 0;
        }
        self.dialog = Some(Dialog::GoLive);
    }

    pub(crate) fn open_settings(&mut self) {
        self.audio_devices = Some(crate::voice::list_devices());
        self.dialog = Some(Dialog::Settings);
    }

    pub(crate) fn watch(&mut self, id: EndpointId) {
        self.view = View::Voice;
        if self.watching != Some(id) {
            self.node.watch(id);
        }
    }

    pub(crate) fn toggle_fullscreen(&mut self, ctx: &egui::Context) {
        self.fullscreen = !self.fullscreen;
        ctx.send_viewport_cmd(ViewportCommand::Fullscreen(self.fullscreen));
    }

    fn room_ui(&mut self, ui: &mut egui::Ui) {
        if self.fullscreen && self.watching.is_some() {
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(c::BG_STAGE))
                .show(ui, |ui| self.stage(ui));
            return;
        }
        self.sidebar(ui);
        if self.view == View::Voice && self.stage_chat {
            egui::Panel::right("stage_chat")
                .exact_size(340.0)
                .resizable(false)
                .frame(egui::Frame::new().fill(c::BG_CONTENT))
                .show(ui, |ui| self.chat_view(ui, true));
        }
        let fill = if self.view == View::Voice { c::BG_STAGE } else { c::BG_CONTENT };
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(fill))
            .show(ui, |ui| match self.view {
                View::Chat => self.chat_view(ui, false),
                View::Voice => self.stage(ui),
            });
    }

    /// Checks for updates on start and every 6 hours (not in demo runs).
    fn poll_update(&mut self, ctx: &egui::Context) {
        if std::env::var_os("BIBICORD_NO_UPDATE").is_some() {
            return;
        }
        let due = self.update_checked.is_none_or(|t| t.elapsed() > Duration::from_secs(6 * 3600));
        if due && (self.demo.is_none() || std::env::var_os("BIBICORD_DEMO_UPDATE").is_some()) {
            self.update_checked = Some(Instant::now());
            crate::update::spawn_check(self.update.clone(), ctx.clone());
        }
    }

    /// The release on offer, if any.
    pub(crate) fn update_available(&self) -> Option<crate::update::Release> {
        use crate::update::State;
        match &*self.update.lock().unwrap() {
            State::Available(r) | State::Downloading(r, _) | State::Installing(r) => Some(r.clone()),
            _ => None,
        }
    }

    pub(crate) fn start_update(&mut self, ctx: &egui::Context) {
        use crate::update::State;
        let r = match &*self.update.lock().unwrap() {
            State::Available(r) => r.clone(),
            _ => return,
        };
        crate::update::spawn_install(self.update.clone(), r, ctx.clone());
    }

    fn run_demo(&mut self, ctx: &egui::Context) {
        // Scripted update test: install as soon as an update is offered.
        if std::env::var_os("BIBICORD_DEMO_AUTOUPDATE").is_some()
            && matches!(*self.update.lock().unwrap(), crate::update::State::Available(_))
        {
            self.start_update(ctx);
        }
        let Some(mut demo) = self.demo.take() else { return };
        let d = &mut demo;
        ctx.request_repaint_after(Duration::from_millis(200));
        if self.screen == Screen::Start && self.net_ready && (d.create.is_some() || d.join.is_some()) {
            if let Some(n) = d.name.take() {
                self.name = n;
            }
            if d.create.is_some() {
                let room = d.room.clone().unwrap_or_else(|| format!("Pokój {}", self.name));
                self.node.create_room(self.name.clone(), room);
                self.enter_room();
            } else if let Some(f) = &d.join {
                if let Ok(t) = std::fs::read_to_string(f) {
                    if self.node.join(&t, self.name.clone()).is_ok() {
                        self.enter_room();
                    }
                }
            }
        } else if self.screen == Screen::Start {
            if let Some(n) = &d.name {
                self.name = n.clone();
            }
        }
        if let (Some(f), Some(t), false) = (&d.create, &self.ticket, d.ticket_written) {
            let _ = std::fs::write(f, t);
            d.ticket_written = true;
        }
        if self.screen == Screen::Room && !d.setup_done {
            d.setup_done = true;
            if d.voice {
                self.join_voice();
            }
            if d.mute {
                self.set_mute(MuteState { muted: true, deafened: false });
            }
            if d.view_voice {
                self.view = View::Voice;
            }
            self.stage_chat = d.stage_chat;
        }
        if !d.chat_sent && !self.peers.is_empty() {
            d.chat_sent = true;
            for line in d.chat.drain(..) {
                self.node.send_chat(line);
            }
        }
        if d.stream && !d.stream_started && !self.peers.is_empty() && crate::ffmpeg::path().is_some() {
            d.stream_started = true;
            self.node.start_stream(self.settings.clone());
        }
        if d.watch && self.watching.is_none() {
            if let Some(id) = self.peers.iter().find(|p| p.stream.is_some()).map(|p| p.id) {
                self.watch(id);
            }
        }
        if !d.dialog_opened && Instant::now() >= d.shot_at - Duration::from_millis(1500) {
            d.dialog_opened = true;
            match d.dialog.as_deref() {
                Some("golive") => self.open_go_live(),
                Some("settings") => self.open_settings(),
                Some("invite") => self.dialog = Some(Dialog::Invite),
                Some("update") => self.dialog = Some(Dialog::Update),
                _ => {}
            }
        }
        if !d.shot_requested && Instant::now() >= d.shot_at {
            if let Some(path) = d.shot.clone() {
                d.shot_requested = true;
                let c = ctx.clone();
                ctx.send_viewport_cmd(ViewportCommand::Screenshot(egui::ScreenshotCallback::new(
                    move |img| {
                        crate::demo::save_png(&path, &img);
                        if std::env::var_os("BIBICORD_DEMO_KEEP").is_none() {
                            c.send_viewport_cmd(ViewportCommand::Close);
                        }
                        c.request_repaint();
                    },
                )));
            }
        }
        self.demo = Some(demo);
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.handle_events(&ctx);
        self.run_demo(&ctx);
        self.poll_update(&ctx);
        if self.chat_visible() {
            self.unread = 0;
        }
        if self.fullscreen && ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.toggle_fullscreen(&ctx);
        }
        if self.fullscreen && self.watching.is_none() {
            self.toggle_fullscreen(&ctx);
        }
        match self.screen {
            Screen::Start => {
                self.start_screen(ui);
                self.dialogs(&ctx);
            }
            Screen::Room => {
                self.room_ui(ui);
                self.dialogs(&ctx);
                if self.my_voice || self.my_stream.is_some() || !self.peers.is_empty() {
                    // Speaking rings, viewer counts and connection paths.
                    ctx.request_repaint_after(Duration::from_millis(100));
                }
            }
        }
        if !self.ffmpeg_status.lock().unwrap().is_empty() {
            ctx.request_repaint_after(Duration::from_millis(500));
        }
        self.toast(&ctx);
    }

    fn on_exit(&mut self) {
        self.node.shutdown();
    }
}
